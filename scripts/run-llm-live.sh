#!/bin/bash
set -euo pipefail

if [ -z "${OPENAI_API_KEY:-}" ]; then
    echo "OPENAI_API_KEY not set; skipping live LLM test"
    exit 0
fi

echo "Running real-LLM integration test against OpenAI..."
LOON_TEST_LIVE_OPENAI=1 cargo test --test e2e_openai_live -- --nocapture
