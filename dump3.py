import re,os,glob
def show(f, start=1, end=None):
    lines=open(f).read().splitlines()
    end = end or len(lines)
    for i,l in enumerate(lines[start-1:end], start): print(f"{f}:{i:4d}  {l}")
for f in ["genossi_service/Cargo.toml","genossi_service_impl/Cargo.toml"]:
    show(f,1,30); print("="*60)
print("### main.rs feature gates:")
for line in open("genossi_bin/src/main.rs").read().splitlines():
    if "cfg(" in line or "feature" in line.lower() or "oidc" in line.lower() or "mock" in line.lower(): print(line)
print("### main.rs lines 1-80:")
show("genossi_bin/src/main.rs",1,80)
