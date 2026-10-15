import type { ItemDoc } from "../types";

export const splitDisplayPath = (path: string): { namespace: string; name: string } => {
  const separatorIndex = path.lastIndexOf("::");

  if (separatorIndex === -1) {
    return { namespace: "", name: path };
  }

  return {
    namespace: path.slice(0, separatorIndex + 2),
    name: path.slice(separatorIndex + 2),
  };
};

export const formatSource = (
  item: { source?: { file: string; line_start: number; line_end: number } | null },
  t: (key: string, params?: Record<string, string | number>) => string,
): string => {
  if (!item?.source) return t(":source_unavailable");
  const { file, line_start, line_end } = item.source;
  const range = line_start === line_end ? `${line_start}` : `${line_start}-${line_end}`;
  return `${file}:${range}`;
};

export const moduleDepth = (path: string) => path.split("::").length;
