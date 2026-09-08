import re,os
print("### refs to backend-mock / mock_auth in nix+scripts+docs:")
for root,dirs,files in os.walk("."):
    dirs[:] = [d for d in dirs if d not in (".git","target",".worktrees","genossi-frontend","node_modules")]
    for f in files:
        if not (f.endswith((".nix",".sh",".md"))): continue
        p=os.path.join(root,f)
        if p.startswith("./.planning") and "ROADMAP" not in p: continue
        for i,l in enumerate(open(p,errors="ignore").read().splitlines(),1):
            if "backend-mock" in l or ("mock_auth" in l and (p.endswith(".nix") or p.endswith(".sh"))):
                print(f"{p}:{i:4d}  {l.strip()[:150]}")
print("="*70)
print("### CLAUDE.md docs-freshness table:")
t=open("CLAUDE.md").read().splitlines()
for i,l in enumerate(t,1):
    if "docs/" in l or "Doku" in l or "Drift" in l:
        print(f"CLAUDE.md:{i:4d}  {l[:160]}")
print("="*70)
print("docs/ listing:")
for r,d,fs in os.walk("docs"):
    for f in sorted(fs): print("  ", os.path.join(r,f))
