import fs from 'fs';
import path from 'path';

interface ChangeItem {
  text: string;
  scope?: string;
  breaking?: boolean;
}

interface ChangeGroup {
  category: string;
  items: ChangeItem[];
}

interface VersionEntry {
  version: string;
  date?: string;
  groups: ChangeGroup[];
}

function parseChangelog(): VersionEntry[] {
  const raw = fs.readFileSync(path.join(process.cwd(), '..', 'CHANGELOG.md'), 'utf-8');
  const lines = raw.split('\n');
  const entries: VersionEntry[] = [];
  let current: VersionEntry | null = null;
  let currentGroup: ChangeGroup | null = null;

  for (const line of lines) {
    const versionMatch = line.match(/^## \[(.+?)\](?:\s*-\s*(.+))?/);
    if (versionMatch) {
      if (current) entries.push(current);
      current = {
        version: versionMatch[1],
        date: versionMatch[2]?.trim(),
        groups: [],
      };
      currentGroup = null;
      continue;
    }

    const groupMatch = line.match(/^### (.+)/);
    if (groupMatch && current) {
      const category = groupMatch[1]
        .replace(/<!--.*?-->/g, '')
        .replace(/[\p{Emoji_Presentation}\p{Extended_Pictographic}]/gu, '')
        .trim();
      currentGroup = { category, items: [] };
      current.groups.push(currentGroup);
      continue;
    }

    const itemMatch = line.match(/^- (.+)/);
    if (itemMatch && currentGroup) {
      let text = itemMatch[1];
      let scope: string | undefined;
      let breaking = false;

      const scopeMatch = text.match(/^\*\((.+?)\)\*\s*/);
      if (scopeMatch) {
        scope = scopeMatch[1];
        text = text.slice(scopeMatch[0].length);
      }

      if (text.startsWith('[**breaking**] ')) {
        breaking = true;
        text = text.slice('[**breaking**] '.length);
      }

      currentGroup.items.push({ text, scope, breaking });
    }
  }

  if (current) entries.push(current);
  return entries;
}

function CategoryLabel({ name }: { name: string }) {
  return (
    <span className="text-xs font-medium uppercase tracking-wider text-fd-muted-foreground">
      {name}
    </span>
  );
}

function VersionBadge({ version, date }: { version: string; date?: string }) {
  const isUnreleased = version.toLowerCase() === 'unreleased';
  return (
    <div className="flex items-center gap-3">
      <span
        className={`rounded-md px-2.5 py-1 text-sm font-semibold ${
          isUnreleased
            ? 'bg-fd-muted text-fd-muted-foreground'
            : 'bg-fd-accent text-fd-foreground'
        }`}
      >
        {isUnreleased ? 'Unreleased' : `v${version}`}
      </span>
      {date && (
        <span className="text-sm text-fd-muted-foreground">{date}</span>
      )}
    </div>
  );
}

export function Changelog() {
  const entries = parseChangelog();

  return (
    <div className="space-y-0">
      {entries.map((entry, i) => {
        const isUnreleased = entry.version.toLowerCase() === 'unreleased';
        const isLast = i === entries.length - 1;

        return (
          <div key={entry.version} className="relative flex gap-6">
            {/* Timeline */}
            <div className="flex flex-col items-center pt-1">
              <div
                className={`h-3 w-3 rounded-full border-2 shrink-0 ${
                  isUnreleased
                    ? 'border-fd-muted-foreground bg-transparent'
                    : 'border-fd-foreground bg-fd-foreground'
                }`}
              />
              {!isLast && (
                <div className="w-px grow bg-fd-border" />
              )}
            </div>

            {/* Content */}
            <div className={`pb-10 ${isUnreleased ? 'opacity-60' : ''}`}>
              <VersionBadge version={entry.version} date={entry.date} />

              <div className="mt-4 space-y-4">
                {entry.groups.map((group) => (
                  <div key={group.category}>
                    <CategoryLabel name={group.category} />
                    <ul className="mt-1.5 space-y-1">
                      {group.items.map((item, j) => (
                        <li
                          key={j}
                          className="text-sm text-fd-foreground/80 leading-relaxed"
                        >
                          {item.breaking && (
                            <span className="mr-1.5 rounded bg-red-500/10 px-1.5 py-0.5 text-xs font-medium text-red-400">
                              breaking
                            </span>
                          )}
                          {item.scope && (
                            <span className="mr-1.5 text-fd-muted-foreground">
                              ({item.scope})
                            </span>
                          )}
                          {item.text}
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
