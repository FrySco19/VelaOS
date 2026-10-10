"""M0 control-plane adapter: receives LFS card metadata; returns ranked OIDs only."""
import json
from pathlib import Path
import sys
import tempfile
import lancedb
from lancedb.index import FTS
from lancedb.rerankers import RRFReranker
import pyarrow as pa
import hybrid_recall as spike

request=json.loads(sys.stdin.readline())
assert request["schema"]=="vela.recall.cards.v1"
assert 1<=len(request["cards"])<=100
embedder,_=spike.make_embedder(Path(__file__).parent/".model-cache",False,4)
cards=request["cards"]
texts=[r["card"]["title"]+"\n"+r["card"]["summary"] for r in cards]
vectors=spike.encode(embedder,texts,"passage: ")
rows=[{"oid":r["oid"],"kind":"NetConfig","title":r["card"]["title"],
    "card_text":text,"model_id":spike.EPOCH,"vector":vec} for r,text,vec in zip(cards,texts,vectors)]
schema=pa.schema([*(pa.field(k,pa.string()) for k in spike.META),pa.field("vector",pa.list_(pa.float32(),384))])
with tempfile.TemporaryDirectory(prefix="vela-m0-control-") as backing:
    table=lancedb.connect(backing).create_table("cards",data=pa.Table.from_pylist(rows,schema=schema))
    table.create_index("card_text",config=FTS(base_tokenizer="whitespace",stem=False,remove_stop_words=True,ascii_folding=False))
    vector=spike.encode(embedder,[request["query"]],"query: ")[0]
    hits=(table.search(query_type="hybrid",fts_columns="card_text").vector(vector).text(request["query"])
        .distance_type("cosine").where(spike.scope_filter("NetConfig"),prefilter=True).limit(64)
        .rerank(RRFReranker(K=60,return_score="all")).to_list())
    print(json.dumps({"status":"RankedCandidates","auto_resolve":False,"embedding_epoch":spike.EPOCH,
        "candidates":[{"oid":r["oid"],"score":r["_relevance_score"]} for r in hits]}))
