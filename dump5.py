import re
def show(f, start=1, end=None):
    lines=open(f).read().splitlines()
    end = end or len(lines)
    for i,l in enumerate(lines[start-1:end], start): print(f"{f}:{i:4d}  {l}")
show("genossi_rest/src/session.rs", 1, 100)
print("="*70)
print("### cfg gates in genossi_rest:")
for f in ["genossi_rest/src/lib.rs"]:
    for i,l in enumerate(open(f).read().splitlines(),1):
        if "cfg" in l or "mock" in l.lower() or "oidc" in l.lower() or "session" in l.lower(): print(f"{f}:{i:4d}  {l}")
print("="*70)
print("### start_server middleware selection:")
for i,l in enumerate(open("genossi_rest/src/lib.rs").read().splitlines(),1):
    if "Middleware" in l or "context_extractor" in l or "forbid_unauthenticated" in l or "auth" in l.lower(): print(f"lib.rs:{i:4d}  {l}")
