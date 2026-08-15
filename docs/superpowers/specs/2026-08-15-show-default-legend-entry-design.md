# Opción para ocultar la entrada default en `match`/`case`

## Contexto

Las expresiones MapLibre `match` y `case` requieren siempre un valor por default (el
último elemento del array) que actúa como catch-all para valores que no matchean ningún
caso. Hoy `parse_match` y `parse_case` (`src/common.rs`) agregan incondicionalmente ese
default como una entrada más del `Vec<(label, color)>` devuelto, y todos los renderers
(`fill.rs`, `circle.rs`, `line.rs`, `fill_extrusion.rs`, `symbol.rs`) dibujan una fila por
cada entrada de esa lista.

El único workaround actual es poner `"default": ""` en `metadata.legend` (label vacío) y
un color/borde transparente en la expresión, para que la fila "no se vea". Pero la fila
se sigue reservando: sigue sumando `ROW_HEIGHT` a la altura del SVG, sólo que en blanco.
Cuando el default realmente no debe aparecer en la leyenda (p. ej. zonas sin
clasificación que se pintan transparentes en el mapa), este espacio en blanco es un
defecto visual, no una opción.

## Alcance

- Nueva clave `"show-default"` (booleano) en `metadata.legend`, leída junto a `label`,
  `default` y `custom-labels` (mismo objeto, mismo nivel).
- Aplica a `match` y `case` — las dos expresiones que tienen una entrada catch-all final.
- **No** aplica a `interpolate` ni `step`: son series de stops sin un catch-all
  equivalente; quedan sin cambios.
- Al operar en `common::parse_match` / `common::parse_case` — el punto único donde se
  arma la lista de entradas usada por todos los renderers — el soporte es automático
  para **todos** los tipos de layer que usan estas expresiones (`fill`, `circle`, `line`,
  `fill-extrusion`, `symbol`), sin tocar cada renderer individualmente.
- Retrocompatible: si la clave está ausente, el comportamiento es idéntico al actual
  (se incluye el default), igual que hoy.

## Comportamiento

Nueva función en `src/common.rs`:

```rust
pub fn get_show_default(layer: &Layer) -> Result<bool, LegendError> {
    let legend = get_legend_object(layer)?;
    Ok(legend
        .and_then(|l| l.get("show-default"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}
```

En `parse_match`:

- El bloque final que hoy hace
  `if let Some(default_color) = arr.last()... { result.push((default_label, default_color)) }`
  se condiciona adicionalmente a `get_show_default(layer)?`.
- Si `show-default` es `false`, ese `push` no ocurre: la entrada default no forma parte
  del `Vec` devuelto, por lo tanto no se reserva ninguna fila en el SVG.
- El resto del parseo (valores + colores intermedios) no cambia.

En `parse_case`: mismo tratamiento, en el bloque análogo que agrega el color final
cuando `arr.len()` es par.

`custom-labels`: si `show-default` es `false`, cualquier label adicional puesto para el
default en `custom-labels` simplemente no se consume (no hay error ni advertencia, igual
que hoy cuando sobran labels).

## Cambios de firma

Ninguno. `parse_match` y `parse_case` mantienen su firma; sólo cambia su lógica interna.
No se toca ningún renderer (`fill.rs`, `circle.rs`, etc.) ni `lib.rs`.

## Testing

Tests unitarios nuevos en `src/common.rs` (junto a los existentes de `parse_match` /
`parse_case`):

- `match` con `"show-default": false` → el resultado tiene una entrada menos que el
  mismo caso con la clave ausente; no aparece el color/label default.
- `match` con `"show-default": true` explícito → comportamiento idéntico al actual
  (regresión).
- `match` sin la clave → comportamiento idéntico al actual (regresión, retrocompat).
- Mismos tres casos para `case`.
- `show-default: false` combinado con `custom-labels` que incluye un label de más para
  el default → se ignora sin error.

## Documentación

- `README.md`: agregar fila `show-default` a la tabla de claves de `metadata.legend`
  (junto a `label`, `default`, `custom-labels`), con nota de que aplica a `match`/`case`
  y por defecto es `true`.
- `CHANGELOG.md`: entrada bajo `[Unreleased]` describiendo la nueva opción.

## Fuera de alcance

- Soporte de `show-default` para `interpolate` o `step` (no tienen catch-all).
- Cambiar el comportamiento de `default` (label) — sigue funcionando igual cuando
  `show-default` es `true` o está ausente.
- Ocultar entradas intermedias de `match`/`case` (sólo aplica al catch-all final).
