import { useEffect } from 'react';
import { addMinutes, setLast, setPageContext, PageContext } from './store';

/** Adds 1 minute of study/piano time for each minute this page is open and visible. */
export function useActiveMinutes(kind: 'study' | 'piano') {
  useEffect(() => {
    const id = setInterval(() => { if (document.visibilityState === 'visible') addMinutes(kind, 1); }, 60000);
    return () => clearInterval(id);
  }, [kind]);
}

/** Tells the AI helper what page the student is on, and remembers it for "continue where you left off". */
export function usePage(ctx: PageContext, last?: { kind: 'study' | 'piano'; path: string }) {
  useEffect(() => {
    setPageContext(ctx);
    if (last) setLast(last.kind, last.path, ctx.label);
  }, [ctx.label, ctx.detail, ctx.subject, last?.kind, last?.path]); // eslint-disable-line react-hooks/exhaustive-deps
}
