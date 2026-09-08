import re
def show(f, start=1, end=None):
    lines=open(f).read().splitlines()
    end = end or len(lines)
    for i,l in enumerate(lines[start-1:end], start): print(f"{f}:{i:4d}  {l}")
show("Cargo.toml")
print("="*70)
for f in ["build-backend.sh","build-oidc-backend.sh","gen-backend.sh","deploy-binaries.sh"]:
    print(f"### {f}")
    for i,l in enumerate(open(f).read().splitlines(),1): print(f"  {i:3d}  {l}")
    print("="*60)
