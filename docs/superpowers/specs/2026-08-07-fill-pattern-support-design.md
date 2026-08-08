# Soporte para `fill-pattern` en capas `fill`

## Contexto

MapLibre Style Spec permite que una capa `fill` defina su relleno mediante `fill-pattern`
(nombre de un ícono del sprite, tileado) en vez de (o además de) `fill-color`. Actualmente
`fill.rs` sólo lee `fill-color`; si una capa usa `fill-pattern`, `render_fill` falla con
`LegendError::InvalidJson` por falta de `fill-color`.

La infraestructura de sprites ya existe y se reutiliza sin cambios:
- `MapLibreLegend.sprite_data: Vec<(DynamicImage, Value)>` — spritesheets cargados.
- `common::get_icon_data_url(sprites, icon_name)` — recorta un ícono y devuelve un data URL
  base64, ya usado por `symbol.rs` para `icon-image`.
- El crate `svg` (0.18) expone `svg::node::element::Image`, ya usado en `symbol.rs`.

## Alcance

- Soporta `fill-pattern` como **string fijo** (nombre de ícono). Un único patrón por capa.
- **No** soporta `fill-pattern` como expresión (`match`/`case`) — error explícito.
- **No** implementa tileado real dentro del swatch; el ícono se escala para llenar el
  rectángulo de 30×20px (igual que un ícono normal, no repetido).
- No afecta el renderizado multi-caso de `fill-color` (match/case/interpolate/step);
  ese camino queda intacto.

## Comportamiento

En `render_fill`:

1. Antes de leer `fill-color`, se revisa `paint.get("fill-pattern")`.
2. **Si es un string:**
   - Si `sprite_data` está vacío → `LegendError::InvalidJson("Missing sprite data for 'fill-pattern'")`
     (mismo mensaje/patrón que `symbol.rs` para `icon-image`).
   - Se busca el ícono con `get_icon_data_url(sprite_data, name)`. Si no existe en ningún
     spritesheet, se propaga el error que ya devuelve esa función.
   - Se dibuja el `<rect>` de 30×20 con stroke = `fill-outline-color` (igual que hoy, para
     mantener el borde), y un `<image>` superpuesto en la misma posición/tamaño con el data
     URL del ícono, reemplazando visualmente el color sólido.
   - Si `has_label` es true, se agrega el label de la misma forma que en el resto de
     `render_fill`.
3. **Si `fill-pattern` está presente pero no es un string** (p. ej. una expresión) →
   `LegendError::InvalidJson("fill-pattern expressions are not yet supported")`.
4. **Si `fill-pattern` está ausente** → comportamiento actual sin cambios (usa `fill-color`,
   soporta expresiones multi-caso como hoy).

## Cambios de firma

- `render_fill` gana un parámetro `sprite_data: &[(DynamicImage, Value)]` (mismo tipo que ya
  recibe `render_symbol`).
- En `lib.rs`, `render_layer_svg` pasa `sprite_data` también en la rama `"fill"` del match
  (ya lo hace para `"symbol"`).

## Testing

- Tests unitarios en `fill.rs` (siguiendo el patrón existente en el módulo):
  - `fill-pattern` string con sprite cargado → SVG contiene un `<image>` con el data URL
    esperado.
  - `fill-pattern` string sin `sprite_data` → error.
  - `fill-pattern` string con ícono inexistente en el sprite → error.
  - `fill-pattern` como array/expresión → error explícito de "no soportado".
  - Capas sin `fill-pattern` → comportamiento actual sin cambios (regresión).

## Ejemplo de prueba manual

- Nuevo `example/style7.json`: capa `fill` con `fill-pattern: "circle_11"` (ícono existente
  en el sprite `https://demotiles.maplibre.org/styles/osm-bright-gl-style/sprite`, ya usado
  en los demás ejemplos).
- `example/src/main.rs`: el loop `for i in 1..=6` pasa a `1..=7` para incluir el nuevo
  archivo y generar `combined_7.svg`, permitiendo inspección visual del resultado.

## Fuera de alcance

- `fill-pattern` vía expresión (`match`/`case`) con un patrón distinto por valor.
- Tileado real del patrón dentro del swatch.
- Soporte de `fill-pattern` en otros tipos de capa (`line-pattern`, etc.) — no forma parte
  de este cambio.
