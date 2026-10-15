import type { CfgExpr, Features } from "../types";

export const evaluateCfg = (cfg: CfgExpr, enabled: Features): boolean => {
  switch (cfg.kind) {
    case "True":
      return true;
    case "False":
      return false;
    case "Feature":
      return enabled[cfg.value] !== undefined;
    case "Flag":
      return true;
    case "All":
      return cfg.value.every((c) => evaluateCfg(c, enabled));
    case "Any":
      return cfg.value.some((c) => evaluateCfg(c, enabled));
    case "Not":
      return !evaluateCfg(cfg.value, enabled);
  }
};

export const formatCfg = (cfg?: CfgExpr | null): string => {
  if (!cfg) return "true";

  switch (cfg.kind) {
    case "True":
      return "true";
    case "False":
      return "false";
    case "Feature":
      return `feature="${cfg.value}"`;
    case "Flag":
      return cfg.value;
    case "All":
      return `all(${(cfg.value || []).map(formatCfg).join(", ")})`;
    case "Any":
      return `any(${(cfg.value || []).map(formatCfg).join(", ")})`;
    case "Not":
      return `not(${formatCfg(cfg.value)})`;
    default:
      return "unknown";
  }
};
