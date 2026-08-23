import json, os, urllib.request

spec = json.loads(open("bench/results/humaneval.jsonl", encoding="utf-8").readline())["prompt"]
for i in range(6):
    body = json.dumps({
        "model": "deepseek-v4-flash",
        "messages": [{"role": "user", "content": "Implement the following task. Reply with ONLY one Python code block.\n\n" + spec}],
        "temperature": 0.2, "max_tokens": 600,
    }).encode()
    req = urllib.request.Request(
        "https://api.deepseek.com/chat/completions", data=body,
        headers={"Content-Type": "application/json",
                 "Authorization": "Bearer " + os.environ["DEEPSEEK_API_KEY"]})
    d = json.loads(urllib.request.urlopen(req, timeout=90).read())
    ch = d["choices"][0]
    m = ch["message"]
    content = m.get("content") or ""
    reasoning = m.get("reasoning_content") or ""
    print(f"run{i}: finish={ch.get('finish_reason')!s:<7} "
          f"ctoks={d['usage']['completion_tokens']:<4} "
          f"content_len={len(content):<5} reasoning_len={len(reasoning)}")
