/**
 * DOM `Range` offsets are UTF-16 code units into a text node's data. The
 * backend's split_and_move_block works in `char`s (Unicode scalar values),
 * which only differs from UTF-16 code units for astral characters (most
 * emoji, etc.). Convert before crossing the Tauri boundary.
 */
export function utf16ToCharIndex(str: string, utf16Index: number): number {
  return Array.from(str.slice(0, utf16Index)).length;
}
