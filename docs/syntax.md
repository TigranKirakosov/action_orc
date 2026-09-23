# Syntax
Can be devided on bound and unbound by context.

## Unbound
- `;` - top-level graph line terminator (optional for inline graphs)
    ```rust
        orc!(
            A -> B;
            X -> Y;
        );
    ```
- `->` - sequence
    ```rust
    orc!(A -> B -> C);
    ```
- `:`: variable binding
    ```rust
    orc!(a: Type);
    ```
- `[]` - variable reference
    ```rust
    orc!([a]);
    ```
- `@` - graph-convertable embedding
    ```rust
    orc!(@Race { x, y });
    ```
- `@[Bound]` (splat) or `@[Bound; Bound]` - annotation of embedding's bounds
    ```rust
    orc!(@[Fork] both_fork);
    // or
    orc!(@[Single; Fork] single_fork);
    ```

## Group Bound
- `( )` - grouping
- `,` - between sequence members
    ```rust
    orc!((A, B, C));
    ```
- `|` - between fork members
    ```rust
    orc!((A | B | C));
    ```
- `?` - between selection members
    ```rust
    orc!(Selector -> (A ? B ? C));
    ```
