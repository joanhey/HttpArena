import os
import sys
import multiprocessing


CPU_COUNT = int(multiprocessing.cpu_count())
WRK_COUNT = min(len(os.sched_getaffinity(0)), 128)
WRK_COUNT = max(WRK_COUNT, 4)


if len(sys.argv) < 2:
    print("Usage: exec_app.py <program> [args...]", file=sys.stderr)
    sys.exit(1)

args = sys.argv[1:]     # [ "fastapi", "run", "app.py", "--port", "8080" ]

prog = args[0]

if os.path.basename(prog) in [ 'fastapi', 'uvicorn' ] and '--workers' not in args:
    args += [ "--workers", str(WRK_COUNT) ]

if '/' in prog:
    os.execv(prog, args)
else:
    os.execvp(prog, args)
