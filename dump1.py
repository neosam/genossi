import re,sys
def show(f, start=1, end=None):
    lines=open(f).read().splitlines()
    end = end or len(lines)
    for i,l in enumerate(lines[start-1:end], start): print(f"{f}:{i:4d}  {l}")
show("genossi_bin/Cargo.toml")
print("="*70)
show("genossi_rest/Cargo.toml", 25, 55)
print("="*70)
import glob
p=glob.glob("**/session.rs", recursive=True)
print(p)
show(p[0] if p else "genossi_rest/src/session.rs", 100, 160)
