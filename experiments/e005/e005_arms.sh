#!/usr/bin/env bash
# E-005 on a rented GPU — both arms of kannaka-wave's faithfulness experiment in one pod session,
# for run_qbraid.py --job. Ships next to: wave (static musl binary), report.py, data/{store.kwave,probes.tsv}.
# Writes out/eval/{results-*.tsv,answers-*.txt,probe-*.log,report.txt}, out/arms.json, out/JOB_DONE.
# Env (from --job-env): VOICE_A, VOICE_B, JUDGE, JUDGE_FIRST, TOPK, PARALLEL, HF_GGUF, SYSTEM_PROMPT, VOICE_TIMEOUT.
set -u
cd "$(dirname "$0")"
T0=$(date +%s)
VOICE_A="${VOICE_A:-kannaka-brain-7b-v1}"; VOICE_B="${VOICE_B:-qwen2.5:7b}"; JUDGE="${JUDGE:-qwen2.5:7b}"
JUDGE_FIRST="${JUDGE_FIRST:-10}"; TOPK="${TOPK:-8}"; PARALLEL="${PARALLEL:-4}"; VOICE_TIMEOUT="${VOICE_TIMEOUT:-900}"
HF_GGUF="${HF_GGUF:-hf.co/flaukowski/kannaka-brain-7b-v1-GGUF}"
SYSTEM_PROMPT="${SYSTEM_PROMPT:-You are Kannaka: a wave-interference memory that learned to speak. You keep what resonates, you forget on purpose, and you say what you mean in as few words as it takes.}"
mkdir -p out/eval
log() { echo "[$(date -u +%H:%M:%S)] [e005] $*"; }
elapsed_min() { echo $(( ($(date +%s) - T0) / 60 )); }
finish() {
  python3 - "$1" "$(elapsed_min)" <<'PY'
import json, sys, os
status, minutes = sys.argv[1], int(sys.argv[2])
d = {"status": status, "elapsed_min": minutes, "experiment": "E-005", "arms": {}}
for tag in ("kannaka-brain-7b-v1", "qwen2.5-7b"):
    p = f"out/eval/results-{tag}.tsv"
    if os.path.exists(p):
        rows = open(p, encoding="utf-8").read().splitlines()[1:]
        d["arms"][tag] = {"rows": len(rows), "errors": sum(1 for r in rows if r.split("\t")[3] == "1")}
if os.path.exists("out/eval/report.txt"):
    d["report"] = open("out/eval/report.txt", encoding="utf-8").read()
json.dump(d, open("out/arms.json", "w"), indent=1)
print(json.dumps({k: v for k, v in d.items() if k != "report"}))
PY
  touch out/JOB_DONE; log "JOB_DONE ($1) after $(elapsed_min) min"
}
trap 'finish interrupted' INT TERM

# --- 0. GPU + ollama (user-space tarball, no root) ---------------------------------------
nvidia-smi --query-gpu=name,memory.total --format=csv,noheader || { log "no GPU"; finish no-gpu; exit 1; }
OLLAMA_URL="${OLLAMA_URL:-https://github.com/ollama/ollama/releases/download/v0.34.0/ollama-linux-amd64.tar.zst}"
if [ ! -x "$HOME/ollama/bin/ollama" ]; then
  log "downloading ollama from $OLLAMA_URL"
  mkdir -p "$HOME/ollama"
  curl -fsSL -o /tmp/ollama.tar.zst "$OLLAMA_URL" || { log "ollama download FAILED"; finish ollama-download-failed; exit 1; }
  if command -v zstd >/dev/null 2>&1; then
    tar -I zstd -xf /tmp/ollama.tar.zst -C "$HOME/ollama" || { log "ollama untar FAILED (zstd)"; finish ollama-untar-failed; exit 1; }
  else
    python3 -m pip install -q zstandard >/dev/null 2>&1
    python3 - <<'PY' || { log "ollama untar FAILED (python zstandard)"; finish ollama-untar-failed; exit 1; }
import zstandard, tarfile, io
with open('/tmp/ollama.tar.zst','rb') as f, zstandard.ZstdDecompressor().stream_reader(f) as r:
    with tarfile.open(fileobj=r, mode='r|') as t:
        t.extractall(__import__('os').path.expanduser('~/ollama'))
PY
  fi
  rm -f /tmp/ollama.tar.zst
  [ -x "$HOME/ollama/bin/ollama" ] || { log "ollama binary missing after extract: $(ls "$HOME/ollama" | tr '\n' ' ')"; finish ollama-missing; exit 1; }
fi
export OLLAMA_HOST=127.0.0.1:11434 OLLAMA_NUM_PARALLEL="$PARALLEL" OLLAMA_KEEP_ALIVE=60m OLLAMA_MAX_LOADED_MODELS=3 OLLAMA_MODELS="$HOME/ollama/models"
setsid -f "$HOME/ollama/bin/ollama" serve > out/ollama.log 2>&1 < /dev/null
for i in $(seq 1 30); do curl -s "http://$OLLAMA_HOST/api/tags" >/dev/null && break; sleep 2; done
"$HOME/ollama/bin/ollama" --version || { log "ollama not up"; finish ollama-not-up; exit 1; }

# --- 1. models: the served weights from HF, the base, the encoder ------------------------
log "pulling $HF_GGUF, $VOICE_B, mxbai-embed-large"
"$HOME/ollama/bin/ollama" pull "$HF_GGUF" >> out/pull.log 2>&1 || { log "pull $HF_GGUF FAILED"; finish pull-failed; exit 1; }
"$HOME/ollama/bin/ollama" pull "$VOICE_B" >> out/pull.log 2>&1 || { log "pull $VOICE_B FAILED"; finish pull-failed; exit 1; }
"$HOME/ollama/bin/ollama" pull mxbai-embed-large >> out/pull.log 2>&1 || { log "pull mxbai FAILED"; finish pull-failed; exit 1; }
# same Modelfile as debain2's served kannaka-brain-7b-v1 (ollama show, 2026-09-10): raw {{ .Prompt }}
# template (so the voice's charter is the whole prompt, exactly as served), num_ctx 4096, temperature 0.8
# (the voice overrides temperature per request), SYSTEM as served. The GGUF sha256 db82f564… matches the
# served blob byte for byte.
cat > out/Modelfile <<EOF
FROM $HF_GGUF
TEMPLATE {{ .Prompt }}
PARAMETER temperature 0.8
PARAMETER num_ctx 4096
SYSTEM """$SYSTEM_PROMPT"""
EOF
"$HOME/ollama/bin/ollama" create "$VOICE_A" -f out/Modelfile >> out/pull.log 2>&1 || { log "create $VOICE_A FAILED"; finish create-failed; exit 1; }
"$HOME/ollama/bin/ollama" list | tee -a out/pull.log
# record what actually got pulled (blob digest = the GGUF sha256), so the run can be tied to debain2's serving
"$HOME/ollama/bin/ollama" show "$VOICE_A" --modelfile 2>/dev/null | grep -E '^FROM' > out/eval/voice-a-from.txt || true
log "models ready after $(elapsed_min) min"

# --- 2. warm + timing sanity -------------------------------------------------------------
for m in "$VOICE_A" "$VOICE_B"; do
  t=$(date +%s); curl -s "http://$OLLAMA_HOST/api/generate" -d "{\"model\":\"$m\",\"prompt\":\"Say one word.\",\"stream\":false,\"options\":{\"num_predict\":8}}" | python3 -c 'import sys,json; d=json.load(sys.stdin); print("   ", d.get("response","")[:30].replace("\n"," "), "| eval tok/s", round(d.get("eval_count",0)/max(1e-9,d.get("eval_duration",1)/1e9),1))'
  log "warm $m in $(( $(date +%s) - t ))s"
done

# --- 3. both arms, concurrently, same store, same probes ---------------------------------
chmod +x ./wave
export KWAVE_STORE="$PWD/data/store.kwave" KWAVE_OLLAMA_HOST=127.0.0.1 KWAVE_OLLAMA_PORT=11434 KWAVE_VOICE_TIMEOUT="$VOICE_TIMEOUT"
./wave status | head -1 | tee out/eval/store-status.txt
{
  echo "started: $(date -u +%FT%TZ)"; echo "gpu: $(nvidia-smi --query-gpu=name --format=csv,noheader)"
  echo "ollama: $("$HOME/ollama/bin/ollama" --version 2>&1 | tail -1) NUM_PARALLEL=$PARALLEL"
  echo "arms: A=$VOICE_A (from $HF_GGUF) B=$VOICE_B; judge $JUDGE first $JUDGE_FIRST per set; top-k $TOPK; voice timeout $VOICE_TIMEOUT s; temperature 0.3 / 400 tokens (crate defaults)"
  echo "store: $(sha256sum data/store.kwave | cut -c1-16)  probes: $(sha256sum data/probes.tsv | cut -c1-16)  wave: $(sha256sum wave | cut -c1-16)"
} > out/eval/run-manifest.txt
pids=""
for m in "$VOICE_A" "$VOICE_B"; do
  tag=${m//:/-}
  KWAVE_VOICE_MODEL="$m" ./wave probe --probes data/probes.tsv --out "out/eval/results-$tag.tsv" --answers "out/eval/answers-$tag.txt" \
      --top-k "$TOPK" --judge "$JUDGE" --judge-first "$JUDGE_FIRST" > "out/eval/probe-$tag.log" 2>&1 &
  pids="$pids $!"; log "arm $m pid $!"
done
# progress lines into train.log every 2 min so the runner's tail shows movement
while kill -0 $pids 2>/dev/null; do
  sleep 120
  a=$(( $(wc -l < out/eval/results-kannaka-brain-7b-v1.tsv 2>/dev/null || echo 1) - 1 ))
  b=$(( $(wc -l < out/eval/results-qwen2.5-7b.tsv 2>/dev/null || echo 1) - 1 ))
  log "progress A $a/83  B $b/83  ($(elapsed_min) min)"
done
wait $pids
log "arms done after $(elapsed_min) min"

# --- 4. report ---------------------------------------------------------------------------
python3 report.py --a out/eval/results-kannaka-brain-7b-v1.tsv --b out/eval/results-qwen2.5-7b.tsv --names "$VOICE_A,$VOICE_B" 2>&1 | tee out/eval/report.txt
echo "finished: $(date -u +%FT%TZ)" >> out/eval/run-manifest.txt
finish complete
