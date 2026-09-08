import re
def show(f, start=1, end=None):
    lines=open(f).read().splitlines()
    end = end or len(lines)
    for i,l in enumerate(lines[start-1:end], start): print(f"{f}:{i:4d}  {l}")
show("module.nix", 1, 140)
print("="*70)
show("default.nix")
print("="*70)
show("example-config.nix",1,80)
