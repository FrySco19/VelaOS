"""M0 frozen-corpus gate. Run with the same pinned environment as hybrid_recall.py."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import random
import tempfile
import time
import lancedb
import numpy as np
from lancedb.rerankers import RRFReranker
import hybrid_recall as spike

def search(table, text, vector):
    return (table.search(query_type="hybrid",fts_columns="card_text")
        .vector(vector).text(text).distance_type("cosine")
        .where(spike.scope_filter(),prefilter=True).limit(spike.TOP_K)
        .rerank(RRFReranker(K=60,return_score="all")).to_list())

def main():
    p=argparse.ArgumentParser(); p.add_argument("--rounds",type=int,default=3)
    p.add_argument("--threads",type=int,default=4); args=p.parse_args()
    assert 1<=args.rounds<=10
    base=Path(__file__).parent; raw=(base/"paraphrases-200.json").read_bytes()
    corpus=json.loads(raw); queries=corpus["queries"]
    assert len(queries)==200 and len({r["query"].casefold() for r in queries})==200
    embedder,model=spike.make_embedder(base/".model-cache",False,args.threads)
    observations=[]; latencies=[]; groups=Counter(); correct=Counter()
    with tempfile.TemporaryDirectory(prefix="vela-m0-gate-") as backing:
        table,labels=spike.index_cards(lancedb.connect(backing),embedder)
        for _,text,_ in spike.CASES*2:
            search(table,text,spike.encode(embedder,[text],"query: ")[0])
        for repeat in range(args.rounds):
            ordered=list(queries); random.Random(731+repeat).shuffle(ordered)
            for row in ordered:
                started=time.perf_counter()
                vector=spike.encode(embedder,[row["query"]],"query: ")[0]
                hits=search(table,row["query"],vector)
                elapsed=(time.perf_counter()-started)*1000
                latencies.append(elapsed)
                expected=labels[row["expected_label"]]
                passed=bool(hits) and hits[0]["oid"]==expected
                assert all(h["model_id"]==spike.EPOCH for h in hits)
                if repeat==0:
                    groups[row["expected_label"]]+=1; correct[row["expected_label"]]+=passed
                    observations.append({**row,"expected_oid":expected,"passed":passed,
                        "top_oid":hits[0]["oid"] if hits else None,
                        "top_title":hits[0]["title"] if hits else None,"elapsed_ms":elapsed})
                elif not passed:
                    assert any(o["id"]==row["id"] and not o["passed"] for o in observations),"ranking changed between rounds"
    recall=sum(o["passed"] for o in observations)/200
    p95=float(np.percentile(latencies,95))
    result={"corpus_sha256":hashlib.sha256(raw).hexdigest(),"unique_queries":200,
        "cards":5,"synthetic_corpus":True,"human_heldout":False,
        "recall_at_1":recall,"groups":{k:{"correct":correct[k],"total":v} for k,v in groups.items()},
        "latency":{"samples":len(latencies),"rounds":args.rounds,"p50_ms":float(np.percentile(latencies,50)),"p95_ms":p95,
            "max_ms":max(latencies),"includes":"single-query embedding + native hybrid retrieval + result materialization",
            "excludes":"model loading, index construction, cold-start and extra diagnostics","threads":args.threads},
        "thresholds":{"recall_at_1_strictly_greater_than":0.9,"p95_ms_strictly_less_than":50},
        "m0_gate_passed":recall>0.9 and p95<50,"model":model,"queries":observations}
    (base/"gate-results.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({k:v for k,v in result.items() if k not in ("model","queries")},indent=2))
    for o in observations:
        if not o["passed"]: print("FAIL",o["id"],o["query"],"=>",o["top_title"])
    raise SystemExit(0 if result["m0_gate_passed"] else 1)
if __name__=="__main__": main()
