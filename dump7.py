import os
print("planning files:")
for r,d,fs in os.walk(".planning"):
    for f in sorted(fs): print(" ", os.path.join(r,f))
print("="*70)
if os.path.exists(".planning/ROADMAP.md"):
    t=open(".planning/ROADMAP.md").read()
    i=t.find("999.1")
    print(t[max(0,i-2000):i+3000] if i>=0 else "999.1 not found in ROADMAP")
print("="*70)
print("module-fixed.nix (1-40):")
print("\n".join(open("module-fixed.nix").read().splitlines()[:40]))
