import Link from 'next/link';

export default function HomePage() {
  return (
    <div className="flex flex-col justify-center text-center flex-1 gap-4">
      <h1 className="text-4xl font-bold">wf</h1>
      <p className="text-lg text-muted-foreground">
        Parameterized shell commands. Interactive prompts. Copied to your clipboard.
      </p>
      <p className="text-sm text-muted-foreground italic">
        Built with event sourcing, CQRS, and an actor model — because why not.
      </p>
      <div className="flex gap-4 justify-center mt-4">
        <Link
          href="/docs"
          className="inline-flex items-center px-4 py-2 rounded-md border border-black bg-black text-white dark:border-white dark:bg-white dark:text-black font-medium hover:opacity-80 transition-opacity"
        >
          Read the Docs
        </Link>
        <Link
          href="https://github.com/sagoez/workflow"
          className="inline-flex items-center px-4 py-2 rounded-md border font-medium hover:bg-accent transition-colors"
        >
          GitHub
        </Link>
      </div>
    </div>
  );
}
