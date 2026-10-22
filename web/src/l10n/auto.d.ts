// l10n-auto-bind（vite.config.ts）会在构建期向每个 .svelte 组件注入：
//   import { l10nDerive } from "./l10n";
//   $: l10n = $l10nDerive("<相对路径>");
// 此声明仅为编辑器提供类型，不参与运行时。
declare const l10n: (key: string, params?: Record<string, string | number>) => string;
