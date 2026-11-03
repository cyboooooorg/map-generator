// The shape every translation follows: the English catalog with its string
// literals widened to `string`, so a translation must provide every message
// (and only those), with the same parameters.

/** A catalog: nested groups of messages, each a string or a function of parameters. */
export type Widen<T> = T extends string
  ? string
  : T extends (...args: infer A) => string
    ? (...args: A) => string
    : { readonly [K in keyof T]: Widen<T[K]> };
