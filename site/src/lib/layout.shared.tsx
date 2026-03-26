import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { gitConfig } from './shared';

function WfIcon() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width={28} height={28}>
      <rect width="64" height="64" rx="14" className="fill-black dark:fill-white" />
      <text
        x="32"
        y="44"
        textAnchor="middle"
        fontFamily="system-ui, -apple-system, sans-serif"
        fontWeight="900"
        fontStyle="italic"
        fontSize="32"
        className="fill-white dark:fill-black"
      >
        wf
      </text>
    </svg>
  );
}

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: <WfIcon />,
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
  };
}
