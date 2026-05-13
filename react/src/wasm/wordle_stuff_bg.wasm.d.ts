/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const black_tile: () => number;
export const yellow_tile: () => number;
export const green_tile: () => number;
export const word_count: () => number;
export const score_word: (a: number, b: number, c: number, d: number) => [number, number, number, number];
export const parse_match: (a: number, b: number) => [number, number, number, number];
export const match_exists: (a: any) => [number, number, number];
export const hard_mode_match_exists: (a: any) => [number, number, number];
export const hard_mode_match_exists_with_progress: (a: any, b: any) => [number, number, number];
export const hard_mode_match_exists_with_candidates: (a: any, b: any) => [number, number, number];
export const generate_trials: (a: number, b: number, c: number, d: bigint, e: number) => [number, number, number];
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_start: () => void;
