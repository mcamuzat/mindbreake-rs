// Official card images, when downloaded locally (scripts/fetch-official-art.sh).
// Without them the page falls back to text cards: nothing depends on them.

interface Manifest {
  /** Normalized name → path relative to /official/. */
  cards: Record<string, string>;
  mindbugs: string[];
}

let manifest: Manifest | null = null;

const BASE = `${import.meta.env.BASE_URL}official/`;

export async function loadArt(): Promise<void> {
  try {
    const res = await fetch(`${BASE}manifest.json`);
    if (res.ok) manifest = (await res.json()) as Manifest;
  } catch {
    manifest = null;
  }
}

/** Same normalization as the script: lowercase, letters and digits only. */
function normalize(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]/g, "");
}

export function cardArt(name: string): string | null {
  const path = manifest?.cards[normalize(name)];
  return path ? BASE + path : null;
}

export function mindbugArt(index: number): string | null {
  const list = manifest?.mindbugs;
  return list && list.length > 0 ? BASE + list[index % list.length] : null;
}
