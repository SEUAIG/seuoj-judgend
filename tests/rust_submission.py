"""Run inside a disposable judge image with --network none (never production).

docker run --rm --network none --entrypoint python3 IMAGE /tmp/rust_submission.py
Copy this script into the disposable container first, or mount it read-only.
Starts its own runner, callback receiver and temporary problem/submission trees.
"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import Request, build_opener, ProxyHandler
from urllib.error import HTTPError


callbacks = {}
checks = []
client = build_opener(ProxyHandler({}))


class Callback(BaseHTTPRequestHandler):
    def do_PUT(self):
        callbacks[self.path.rsplit("/", 1)[-1]] = json.loads(
            self.rfile.read(int(self.headers["Content-Length"]))
        )
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"{}")

    def log_message(self, *args):
        pass


def api(path, payload=None, method=None):
    req = Request("http://127.0.0.1:9090" + path,
                  data=None if payload is None else json.dumps(payload).encode(),
                  headers={"Content-Type": "application/json"}, method=method)
    try:
        with client.open(req, timeout=60) as response:
            return json.load(response)
    except HTTPError as error:
        raise AssertionError((path, error.code, error.read().decode())) from error


def record(name, result):
    checks.append({"test": name, "result": result})
    print(json.dumps(checks[-1]), flush=True)


def submit(name, code, language="Rust", pid="RUSTSMOKE", online=False, stdin="1 2\n"):
    payload = {"submissionId": name, "pid": pid, "language": language, "code": code}
    if online:
        payload["testcases"] = [{"id": 1, "in": stdin}]
        result = api("/judge/submission/online", payload)["data"]
    else:
        api("/judge/submission", payload)
        deadline = time.monotonic() + 60
        while name not in callbacks:
            assert time.monotonic() < deadline, (name, "callback timeout")
            time.sleep(0.05)
        result = callbacks[name]
    return result


def expect(name, code, expected="Accepted", **kwargs):
    result = submit(name, code, **kwargs)
    if expected == "CompileError":
        assert result["status"] == expected, (name, result)
        assert result.get("errorDetail"), result
    else:
        assert result["status"] == "Success", (name, result)
        types = [case["type"] for case in result["resultDetail"]]
        assert types and all(t == expected for t in types), (name, result)
        if not kwargs.get("online"):
            assert result["score"] == (100 if expected == "Accepted" else 0), (name, result)
    record(name, result)
    return result


RUST = """use proconio::{input, fastout, derive_readable};
#[derive_readable] struct Pair { a: i64, b: i64 }
#[fastout] fn main() { input! { p: Pair } println!("{}", p.a + p.b); }
"""
CPP = '#include <iostream>\nint main(){long long a,b;std::cin>>a>>b;std::cout<<a+b;}'
C = '#include <stdio.h>\n#ifndef ONLINE_JUDGE\n#error missing macro\n#endif\nint main(){long long a,b;scanf("%lld%lld",&a,&b);printf("%lld",a+b);}'


def run(root):
    languages = api("/judge/languages")["data"]["languages"]
    assert {x["name"] for x in languages if x["available"]} == {
        "C", "Cpp", "Cpp20", "Python", "Nodejs", "Go", "Java", "Rust"
    }, languages
    record("language availability", languages)

    base = {
        "problem_info": {"problem_type": "Standard", "checker_type": "Standard",
                         "time_limit_ms": 1500, "memory_limit_kb": 262144, "max_code_length": 65536},
        "testcases": [{"id": i, "in_path": f"{i}.in", "ans_path": f"{i}.ans", "weight": 50}
                      for i in (1, 2)], "subtasks": [], "custom_modules": None,
    }

    def problem(pid, config, files=None):
        path = root / "problems" / pid
        (path / "data").mkdir(parents=True)
        (path / "problem.md").write_text(f"---\npid: {pid}\n---\n\n## 题目描述\n\nA+B.\n")
        values = {"1.in": "1 2\n", "1.ans": "3\n", "2.in": "-9 4\n", "2.ans": "-5\n"}
        values.update(files or {})
        for filename, content in values.items():
            (path / "data" / filename).write_text(content)
        result = api("/judge/problem/config/" + pid, config, "PUT")
        assert result["code"] == 0, result

    problem("RUSTSMOKE", base)
    codes = {
        "Rust": RUST, "C": C, "Cpp": CPP,
        "Cpp20": '#include <concepts>\nstatic_assert(std::integral<int>);\n' + CPP,
        "Python": 'a,b=map(int,input().split());print(a+b)',
        "Nodejs": 'const [a,b]=require("fs").readFileSync(0,"utf8").trim().split(/\\s+/).map(Number);console.log(a+b)',
        "Go": 'package main\nimport "fmt"\nfunc main(){var a,b int64;fmt.Scan(&a,&b);fmt.Println(a+b)}',
        "Java": 'import java.util.*;public class Main{public static void main(String[]args){Scanner s=new Scanner(System.in);System.out.println(s.nextLong()+s.nextLong());}}',
    }
    for language, code in codes.items():
        expect("standard-" + language, code, language=language)
        result = expect("online-" + language, code, language=language, online=True)
        assert result["resultDetail"][0]["out"].strip() == "3", result

    expect("rust-wrong-answer", 'fn main(){println!("0");}', "WrongAnswer")
    expect("rust-compile-error", "fn main() { missing_symbol(); }", "CompileError")
    expect("rust-unavailable-crate", "use itertools::Itertools; fn main() {}", "CompileError")
    expect("rust-panic", 'fn main(){panic!("expected test panic");}', "RuntimeError", online=True)
    result = expect("rust-stdlib", '''use std::{collections::HashMap, io::{self,Read}};
fn main(){let mut s=String::new();io::stdin().read_to_string(&mut s).unwrap();
let v:Vec<i64>=s.split_whitespace().map(|x|x.parse().unwrap()).collect();
let mut m=HashMap::new();m.insert("sum",v.iter().sum::<i64>());println!("{}",m["sum"]);}''')
    markers = '''use proconio::{input,marker::{Chars,Usize1}};
fn main(){input!{n:usize,a:[i64;n],s:Chars,k:Usize1}println!("{} {} {}",a.iter().sum::<i64>(),s.len(),k);}'''
    result = expect("rust-markers", markers, online=True, stdin="3 2 4 6 abc 2\n")
    assert result["resultDetail"][0]["out"].strip() == "12 3 1", result
    expect("rust-online-judge-cfg", '#[cfg(not(ONLINE_JUDGE))] compile_error!("missing cfg");\n' + RUST)

    limited = copy.deepcopy(base)
    limited["problem_info"].update(time_limit_ms=100, memory_limit_kb=65536)
    problem("RUSTLIMIT", limited)
    expect("rust-time-limit", 'fn main(){loop{std::hint::black_box(1);}}',
           "TimeLimitExceeded", pid="RUSTLIMIT", online=True)
    memory = submit("rust-memory-limit", 'fn main(){let v=vec![1u8;268435456];std::hint::black_box(v);}',
                    pid="RUSTLIMIT", online=True)
    assert memory["status"] == "Success", memory
    assert memory["resultDetail"][0]["type"] in ("MemoryLimitExceeded", "RuntimeError"), memory
    record("rust-memory-limit", memory)
    for name, code in {
        "file-write": 'fn main(){std::fs::write("/tmp/rust-forbidden-write","x").unwrap();}',
        "network": 'fn main(){std::net::TcpStream::connect("127.0.0.1:8080").unwrap();}',
        "thread": 'fn main(){std::thread::spawn(||()).join().unwrap();}',
    }.items():
        expect("rust-block-" + name, code, "RuntimeError", online=True)
    assert not Path("/tmp/rust-forbidden-write").exists()

    grader = copy.deepcopy(base)
    grader["custom_modules"] = {"grader_paths": ["grader.cpp"]}
    problem("RUSTGRADER", grader, {"grader.cpp": '#include <iostream>\nlong long add(long long,long long);int main(){long long a,b;std::cin>>a>>b;std::cout<<add(a,b);} '})
    result = expect("rust-grader-rejected", RUST, "CompileError", pid="RUSTGRADER")
    assert "grader" in result["errorDetail"], result
    expect("cpp-grader-regression", 'long long add(long long a,long long b){return a+b;}', language="Cpp", pid="RUSTGRADER")

    special = copy.deepcopy(base)
    special["problem_info"].update(problem_type="Special", checker_type="Special")
    special["custom_modules"] = {"checker_path": "checker"}
    checker = '#!/usr/bin/python3\nimport sys\na=open(sys.argv[2]).read().split();b=open(sys.argv[3]).read().split();sys.exit(0 if a==b else 1)\n'
    problem("RUSTSPECIAL", special, {"checker": checker})
    for language, code in (("Rust", RUST), ("Cpp", CPP)):
        expect("special-" + language, code, language=language, pid="RUSTSPECIAL")

    interactive = copy.deepcopy(base)
    interactive["problem_info"].update(problem_type="Interactive", checker_type="Interactor")
    interactive["custom_modules"] = {"interactor_path": "interactor"}
    interactor = '''#!/usr/bin/python3
import sys
a,b=map(int,open(sys.argv[1]).read().split())
print(a,b,flush=True)
answer=sys.stdin.readline().strip()
open(sys.argv[2],"w").write(answer)
if answer!=str(a+b):
    print("wrong interactive answer",file=sys.stderr)
    sys.exit(1)
'''
    problem("RUSTINTERACTIVE", interactive, {"interactor": interactor})
    rust_interactive = '''use proconio::{input,source::line::LineSource};
use std::io::{self,Write};
fn main(){let stdin=io::stdin();let mut source=LineSource::new(stdin.lock());
input!{from &mut source,a:i64,b:i64}println!("{}",a+b);io::stdout().flush().unwrap();}'''
    cpp_interactive = CPP.replace("std::cout<<a+b;", "std::cout<<a+b<<std::endl;")
    for language, code in (("Rust", rust_interactive), ("Cpp", cpp_interactive)):
        expect("interactive-" + language, code, language=language, pid="RUSTINTERACTIVE")

    subtasks = copy.deepcopy(base)
    subtasks["subtasks"] = [{"id": 1, "cases": [1, 2], "score": 100, "type": "min", "pre_subtasks": []}]
    problem("RUSTSUBTASK", subtasks)
    expect("rust-subtasks", RUST, pid="RUSTSUBTASK")
    print(json.dumps({"passed": len(checks)}), flush=True)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="rust-judge-smoke-") as temp:
        root = Path(temp)
        callback = ThreadingHTTPServer(("127.0.0.1", 8080), Callback)
        threading.Thread(target=callback.serve_forever, daemon=True).start()
        env = dict(os.environ, AIJ_PROBLEMS_DIR=str(root / "problems"),
                   AIJ_SUBMISSIONS_DIR=str(root / "submissions"), AIJ_LOG_DIR=str(root / "logs"),
                   AIJ_BACKEND_HOST="127.0.0.1", AIJ_BACKEND_PORT="8080", AIJ_BACKEND_PREFIX="",
                   AIJ_COMPILE_TIME_LIMIT_MS="20000", AIJ_COMPILE_MEMORY_LIMIT_KB="1048576")
        with (root / "runner.log").open("w") as log:
            runner = subprocess.Popen(["/app/runner"], env=env, stdout=log, stderr=log)
            try:
                for attempt in range(100):
                    try:
                        api("/judge/languages")
                        break
                    except OSError:
                        assert runner.poll() is None, "runner exited"
                        time.sleep(0.1)
                run(root)
            except Exception:
                print((root / "runner.log").read_text()[-12000:])
                raise
            finally:
                runner.terminate()
                runner.wait(timeout=10)
                callback.shutdown()
