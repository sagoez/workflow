import Link from 'next/link';
import fs from 'fs';
import path from 'path';

export default function HomePage() {
  const pkg = JSON.parse(fs.readFileSync(path.join(process.cwd(), 'package.json'), 'utf-8'));
  const version = pkg.version;
  return (
    <div className="flex flex-col justify-center text-center flex-1 gap-6 px-6">
      <div className="space-y-4" style={{ animation: 'fadeInUp 0.5s ease-out' }}>
        <div className="inline-flex mx-auto px-3 py-1 rounded-full border border-fd-border text-xs text-fd-muted-foreground tracking-wide uppercase">
          v{version}
        </div>
        <h1 className="text-5xl font-bold tracking-tight">wf</h1>
        <p className="text-lg text-fd-muted-foreground max-w-md mx-auto leading-relaxed">
          Parameterized shell commands. Interactive prompts. Copied to your clipboard.
        </p>
        <p className="text-sm text-fd-muted-foreground/60 italic">
          Built with event sourcing, CQRS, and an actor model — because why not.
        </p>
      </div>

      <div
        className="flex gap-3 justify-center mt-2"
        style={{ animation: 'fadeInUp 0.5s ease-out 0.1s both' }}
      >
        <Link
          href="/docs"
          className="inline-flex items-center px-5 py-2.5 rounded-lg bg-fd-foreground text-fd-background font-medium text-sm hover:opacity-90 active:scale-[0.98] transition-all"
        >
          Documentation
        </Link>
        <Link
          href="https://github.com/sagoez/workflow-vault"
          className="inline-flex items-center px-5 py-2.5 rounded-lg border border-fd-border font-medium text-sm hover:bg-fd-accent active:scale-[0.98] transition-all"
        >
          Workflow Vault
        </Link>
        <Link
          href="https://github.com/sagoez/workflow"
          className="inline-flex items-center px-5 py-2.5 rounded-lg border border-fd-border font-medium text-sm hover:bg-fd-accent active:scale-[0.98] transition-all"
        >
          GitHub
        </Link>
      </div>

      <div
        className="mt-8 mx-auto w-full max-w-lg rounded-xl border border-fd-border bg-fd-card p-4 text-left font-mono text-sm"
        style={{ animation: 'fadeInUp 0.5s ease-out 0.2s both' }}
      >
        <div className="flex items-center gap-2 mb-3">
          <span className="w-3 h-3 rounded-full bg-red-500/70" />
          <span className="w-3 h-3 rounded-full bg-yellow-500/70" />
          <span className="w-3 h-3 rounded-full bg-green-500/70" />
          <span className="ml-2 text-xs text-fd-muted-foreground">terminal</span>
        </div>
        <div className="space-y-1 text-fd-muted-foreground">
          <p><span className="text-fd-foreground">$</span> cargo install wf-cli</p>
          <p><span className="text-fd-foreground">$</span> wf</p>
          <p className="text-fd-foreground/60">  › Select workflow</p>
          <p className="text-fd-foreground/60">  ? Enter port <span className="text-cyan-400">8080</span></p>
          <p className="mt-2 text-green-400/80">  Copied to clipboard</p>
        </div>
      </div>
    </div>
  );
}
