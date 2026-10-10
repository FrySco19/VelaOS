# /// script
# requires-python = ">=3.12,<3.13"
# dependencies = [
#   "lancedb==0.40.0", "fastembed==0.9.0", "blake3==1.0.8",
#   "rfc8785==0.1.4", "huggingface-hub==1.33.0", "numpy==2.5.3",
#   "pyarrow==26.0.0", "onnxruntime==1.31.0", "tokenizers==0.23.3",
# ]
# ///
"""M0 only: real embeddings + Lance-native BM25 + RRF(60), over five cards.

No planner, payload resolver, authority token, route, or data-plane implementation.
The temporary database is experiment infrastructure, not an LFS namespace.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
from importlib.metadata import version
import json
from pathlib import Path
import platform
import re
import tempfile
import time

from blake3 import blake3
from fastembed import TextEmbedding
from fastembed.common.model_description import ModelSource, PoolingType
from huggingface_hub import snapshot_download
import lancedb
from lancedb.index import FTS
from lancedb.rerankers import RRFReranker
import numpy as np
import pyarrow as pa
import rfc8785

MODEL = "intfloat/multilingual-e5-small"
REVISION = "614241f622f53c4eeff9890bdc4f31cfecc418b3"
MODEL_SHA256 = "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665"
MODEL_HASHES = {
    "onnx/model.onnx": MODEL_SHA256,
    "config.json": "69137736cab8b8903a07fe8afaafdda25aac55415a12a55d1bffa9f581abf959",
    "tokenizer.json": "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39",
    "tokenizer_config.json": "a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b",
    "special_tokens_map.json": "d05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7",
}
EPOCH = f"{MODEL}@{REVISION}:onnx-fp32:mean:l2:query-passage:v1"
DIM = 384
RRF_K = 60
TOP_K = 64
MODEL_FILES = ["onnx/model.onnx", "config.json", "tokenizer.json",
               "tokenizer_config.json", "special_tokens_map.json"]
META = ["oid", "title", "kind", "card_text", "model_id"]

# Labels are test annotations only. They never enter a search/ranking function.
CARDS = [
    {"label": "exact_profile", "kind": "NetConfig", "title": "Recorded configuration",
     "summary": "Archived configuration state for an earlier network session.",
     "entities": {"SSID": "ZEPHYR-731", "DNS": "9.9.9.9",
                  "UUID": "8b9539b7-3c6f-4bb1-9355-ae58471cf013"}},
    {"label": "semantic_profile", "kind": "NetConfig", "title": "Espresso wireless session",
     "summary": "A laptop connects over encrypted Wi-Fi at a coffee shop. A secure tunnel towards Milan protects traffic on the public hotspot.",
     "entities": {"SSID": "CafeGuest-5G", "DNS": "1.1.1.1"}},
    {"label": "home_profile", "kind": "NetConfig", "title": "Home entertainment connection",
     "summary": "A home router serves the television and game console with automatic addresses. No secure tunnel is enabled.",
     "entities": {"SSID": "HomeLan-5G", "DNS": "8.8.8.8"}},
    {"label": "guide", "kind": "Note", "title": "Wireless troubleshooting handbook",
     "summary": "Instructions for diagnosing a network name, resolver and connectivity. This is a guide, not a saved configuration.",
     "entities": {"topic": "connectivity"}},
    {"label": "media", "kind": "Media", "title": "Ferry at sunset",
     "summary": "A short film of a sailboat crossing the bay in golden evening light.",
     "entities": {"topic": "sailing"}},
]
CASES = [
    ("exact_ssid", "ZEPHYR-731", "exact_profile"),
    ("exact_ip", "9.9.9.9", "exact_profile"),
    ("exact_uuid", "8b9539b7-3c6f-4bb1-9355-ae58471cf013", "exact_profile"),
    ("semantic_it", "connessione protetta del portatile in caffetteria", "semantic_profile"),
    ("lexical_vs_semantic", "ZEPHYR-731 connessione protetta del portatile in caffetteria", "exact_profile"),
    ("semantic_media_it", "filmato della barca al tramonto", "media"),
]


def oid_of(payload: dict) -> str:
    """I1: RFC 8785 canonical plaintext, NOT title/vector/card as identity."""
    return blake3(rfc8785.dumps(payload)).hexdigest()


def card_text(card: dict) -> str:
    # Pure fixture extractor, no LLM. Identifiers remain whitespace-delimited tokens.
    return "\n".join([card["title"], card["summary"],
                      *[f"{key} {value}" for key, value in sorted(card["entities"].items())]])


def make_embedder(cache: Path, download: bool, threads: int):
    started = time.perf_counter()
    snapshot = Path(snapshot_download(MODEL, revision=REVISION,
                    allow_patterns=MODEL_FILES, cache_dir=str(cache),
                    local_files_only=not download))
    h = hashlib.sha256()
    with (snapshot / "onnx/model.onnx").open("rb") as stream:
        for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            h.update(block)
    if h.hexdigest() != MODEL_SHA256:
        raise RuntimeError("Pinned embedding weights failed SHA256 verification")
    checksums = {name: hashlib.sha256((snapshot / name).read_bytes()).hexdigest()
                 for name in MODEL_FILES if name != "onnx/model.onnx"}
    checksums["onnx/model.onnx"] = h.hexdigest()
    if checksums != MODEL_HASHES:
        raise RuntimeError("Pinned tokenizer/config files failed SHA256 verification")
    # FastEmbed supports custom mean-pooling ONNX models. No fallback/substitution.
    if not any(m["model"] == MODEL for m in TextEmbedding.list_supported_models()):
        TextEmbedding.add_custom_model(MODEL, PoolingType.MEAN, True,
            ModelSource(hf=MODEL), DIM, model_file="onnx/model.onnx", license="mit")
    embedder = TextEmbedding(MODEL, specific_model_path=str(snapshot),
                            local_files_only=True, providers=["CPUExecutionProvider"],
                            threads=threads)
    return embedder, {"epoch": EPOCH, "revision": REVISION, "sha256": checksums,
                      "setup_seconds": time.perf_counter() - started}


def encode(embedder, texts: list[str], prefix: str) -> list[list[float]]:
    vectors = np.asarray(list(embedder.embed([prefix + t for t in texts], batch_size=8)),
                         dtype=np.float32)
    if vectors.shape != (len(texts), DIM) or not np.isfinite(vectors).all():
        raise RuntimeError("Invalid embedding shape or values")
    vectors /= np.linalg.norm(vectors, axis=1, keepdims=True)
    return vectors.tolist()


def index_cards(db, embedder):
    texts = [card_text(c) for c in CARDS]
    vectors = encode(embedder, texts, "passage: ")
    rows, labels = [], {}
    for card, text, vector in zip(CARDS, texts, vectors):
        # Typed synthetic object. Semantic metadata stays outside canonical plaintext.
        payload = {"schema": "vela.spike." + card["kind"].lower() + ".v1",
                   "state": card["entities"]}
        oid = oid_of(payload)
        labels[card["label"]] = oid
        rows.append({"oid": oid, "kind": card["kind"], "title": card["title"],
                     "card_text": text, "model_id": EPOCH, "vector": vector})
    schema = pa.schema([*(pa.field(k, pa.string()) for k in META),
                        pa.field("vector", pa.list_(pa.float32(), DIM))])
    table = db.create_table("semantic_cards", data=pa.Table.from_pylist(rows, schema=schema))
    table.create_index("card_text", config=FTS(base_tokenizer="whitespace", stem=False,
                      remove_stop_words=True, ascii_folding=False, max_token_length=128,
                      memory_limit=64 * 1024 * 1024, num_workers=2))
    return table, labels


def scope_filter(kind: str | None = None) -> str:
    """Fixture filter only; not a Biscuit authorizer and grants no authority."""
    if kind not in (None, "NetConfig", "Note", "Media"):
        raise ValueError("Unknown typed kind")
    expr = f"model_id = '{EPOCH}'"
    return expr + (f" AND kind = '{kind}'" if kind else "")


def ranked_search(table, text: str, vector: list[float], kind=None) -> dict:
    predicate = scope_filter(kind)
    dense = (table.search(vector).distance_type("cosine").where(predicate, prefilter=True)
             .select(META + ["_distance"]).limit(TOP_K).to_list())
    lexical = (table.search(text, query_type="fts", fts_columns="card_text")
               .where(predicate, prefilter=True).select(META + ["_score"]).limit(TOP_K).to_list())
    hybrid = (table.search(query_type="hybrid", fts_columns="card_text")
              .vector(vector).text(text).distance_type("cosine")
              .where(predicate, prefilter=True).limit(TOP_K)
              .rerank(RRFReranker(K=RRF_K, return_score="all")).to_list())
    for row in hybrid:
        row.pop("vector", None)  # The public result contains only cards/OIDs/scores.
    expected = defaultdict(float)
    for results in (dense, lexical):
        for rank, row in enumerate(results, 1):
            expected[row["oid"]] += 1 / (RRF_K + rank)
    for row in hybrid:
        if abs(row["_relevance_score"] - expected[row["oid"]]) > 1e-7:
            raise AssertionError("Native fusion differs from RRF(60)")
    assert all(row["model_id"] == EPOCH for rows in (dense, lexical, hybrid) for row in rows)
    assert kind is None or all(row["kind"] == kind for rows in (dense, lexical, hybrid) for row in rows)
    gap = (hybrid[0]["_relevance_score"] - hybrid[1]["_relevance_score"]
           if len(hybrid) > 1 else None)
    return {"status": "RankedCandidates" if hybrid else "Empty", "dense": dense,
            "bm25": lexical, "hybrid": hybrid, "top_gap": gap,
            "auto_resolve": False}  # I4: no confidence calibration or fake handle.


def check_oid_lookup(table, oid: str):
    if not re.fullmatch(r"[0-9a-f]{64}", oid):
        raise ValueError("Invalid content OID")
    rows = table.search().where(f"oid = '{oid}' AND " + scope_filter()).select(META).to_list()
    assert len(rows) == 1 and rows[0]["oid"] == oid
    return rows[0]  # Metadata only. lfs.open/data-plane bytes belong to later milestones.


def run(embedder, repeat: int) -> dict:
    with tempfile.TemporaryDirectory(prefix="vela-m0-") as backing:
        table, labels = index_cards(lancedb.connect(backing), embedder)
        assert table.count_rows() == 5
        observations = []
        for name, query, wanted in CASES:
            start = time.perf_counter()
            vector = encode(embedder, [query], "query: ")[0]
            embedding_ms = (time.perf_counter() - start) * 1000
            start = time.perf_counter()
            found = ranked_search(table, query, vector)
            search_ms = (time.perf_counter() - start) * 1000
            passed = bool(found["hybrid"]) and found["hybrid"][0]["oid"] == labels[wanted]
            observations.append({"name": name, "query": query, "expected_oid": labels[wanted],
                                 "passed": passed, "embedding_ms": embedding_ms,
                                 "search_ms": search_ms, **found})
        contrast = next(r for r in observations if r["name"] == "lexical_vs_semantic")
        assert contrast["dense"][0]["oid"] == labels["semantic_profile"], "Fixture must expose a semantic-first ranking"
        assert contrast["bm25"][0]["oid"] == labels["exact_profile"], "BM25 must recover the exact identifier"
        assert contrast["hybrid"][0]["oid"] == labels["exact_profile"], "Hybrid must demonstrate lexical recovery"
        # I1: reordering keys and changing semantic metadata cannot change identity.
        assert oid_of({"b": 2, "a": 1}) == oid_of({"a": 1, "b": 2})
        check_oid_lookup(table, labels["exact_profile"])
        assert not table.search().where("model_id = 'another-epoch'").to_list()
        filtered = ranked_search(table, "ZEPHYR-731",
                                 encode(embedder, ["ZEPHYR-731"], "query: ")[0], kind="Media")
        assert not filtered["bm25"] and labels["exact_profile"] not in [r["oid"] for r in filtered["hybrid"]]
        # Repeated warm queries are timing samples, NOT 200 independent paraphrases.
        samples = []
        for i in range(repeat):
            _, query, _ = CASES[i % len(CASES)]
            start = time.perf_counter()
            vector = encode(embedder, [query], "query: ")[0]
            ranked_search(table, query, vector)
            samples.append((time.perf_counter() - start) * 1000)
        return {"cards": 5, "candidate_limit_per_branch": TOP_K, "rrf_k": RRF_K,
                "fts": "Lance-native BM25, whitespace tokens, English stopwords, no stemming",
                "vector_search": "exact cosine scan; five rows do not justify an ANN index",
                "cases": observations, "passed": all(r["passed"] for r in observations),
                "checks": {"canonical_oid": True, "exact_oid_metadata_lookup": True,
                           "model_epoch_filter": True, "kind_prefilter_both_branches": True,
                           "native_rrf_verified": True, "no_payloads_returned": True},
                "warm_timing": {"samples": repeat, "includes_embedding_and_three_searches": True,
                                "p50_ms": float(np.percentile(samples, 50)),
                                "p95_ms": float(np.percentile(samples, 95))},
                "m0_roadmap_gate_passed": False,
                "limit": "Six handcrafted queries over five dummy cards, not 200 independent paraphrases. No LFS service, handles, Biscuit, commit log, planner or iceoryx2 data plane."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--download-model", action="store_true", help="Explicit first-run model bootstrap")
    parser.add_argument("--cache", type=Path, default=Path(__file__).parent / ".model-cache")
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "results.json")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--repeat", type=int, default=24)
    args = parser.parse_args()
    if not 1 <= args.repeat <= 1000 or not 1 <= args.threads <= 32:
        parser.error("repeat must be 1..1000 and threads 1..32")
    embedder, model = make_embedder(args.cache, args.download_model, args.threads)
    report = run(embedder, args.repeat)
    report.update(model=model, environment={"python": platform.python_version(),
        "os": platform.system(), "machine": platform.machine(), "cpu": platform.processor(), "threads": args.threads,
        "packages": {p: version(p) for p in ("lancedb", "fastembed", "blake3", "rfc8785", "numpy", "pyarrow", "onnxruntime", "tokenizers")}})
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False) + "\n", encoding="utf-8")
    for case in report["cases"]:
        top = case["hybrid"][0] if case["hybrid"] else {}
        print(f"{'PASS' if case['passed'] else 'FAIL'} {case['name']}: {top.get('title')} [{top.get('oid', '')[:12]}]")
    print("Warm p95 (embedding + diagnostic searches):", round(report["warm_timing"]["p95_ms"], 2), "ms")
    print("M0 roadmap gate: NOT evaluated on 200 independent paraphrases")
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
