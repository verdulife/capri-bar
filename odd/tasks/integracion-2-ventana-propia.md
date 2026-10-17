# Integración 2 — la isla nace en una ventana propia

Capa de ventana propia para Capri Bar: crear el HWND desde el proceso y dibujar la isla dentro con Slint.
Motivo: la ventana debe **nacer** inelegible para un gestor de ventanas con tiling (komorebi es el entorno
objetivo de la v1), y la app no debe depender de la configuración de komorebi de cada usuario.

## Specs

**S1.** La isla debe flotar: komorebi no debe meterla en su layout de tiling, y debe verse a su tamaño real.
> "ahora me ha salido la island, pero se he añade como una ventana/tile en komorebi. tiene que ser flotante"

**S2.** La ventana nace inelegible; no se parchea el estilo después de crearla.
> "no existe una manera de hacer que capri nazca inekegible como pasa con yasb?"

**S3.** `ui/island.slint` no se toca.
> "la regla de que la integración 1 dibuja la isla SIN interacción ni lógica (no añadas funcionalidad, no "mejores" el fichero .slint)"

**S4.** Todo lo de Slint por encima de la capa de ventana se conserva: estilo declarativo, animaciones, callbacks y estado.
> "estaria bien saber si una vez creada la ventana podemos seguir usando slint para estilizarla y o darle funcionalidad, animaciones etc."

**S5.** Criterio de medida de la isla, el mismo de la integración 1: **260 × 44 px**, píxel central `#000000` opaco,
esquina fuera del radio transparente, radio medido **22 px**.

**S6.** Proceso: documento de tareas y spike aislado antes de tocar `src/main.rs`.
> "empezamos por el principio: documento, tareas y spike antes de tocar `src/main.rs`" (acordado con el usuario; aceptado con "ok dale")

**S7.** Nada se commitea ni se publica sin decisión del usuario.
> "No hagas commit ni push: eso lo decido yo."

## Tasks

| ID | Specs | Tarea | Ruta | Commit |
|---|---|---|---|---|
| T1 | S1, S2, S6 | Spike aislado: ventana Win32 creada con estilo propio (sin `WS_CAPTION` ni `WS_EX_WINDOWEDGE`) y comprobación de que komorebi la ignora desde el nacimiento — **verificado**, ver L5 | inline | — |
| T2 | S1–S5 | Portar la plataforma al crate real: dependencias, módulo de plataforma y `src/main.rs` — **verificado**, ver L6 | inline | — |
| T3 | S3, S5 | Verificación de la isla: cuatro medidas del render headless, medida en pantalla y build sin errores ni avisos — **verificado**, ver L6 | inline | — |
| T4 | S1, S5 | Retoque tras la verificación independiente: el tamaño deja de estar duplicado en Rust y sale del `.slint`, se atiende un cambio de escala, se arreglan los tiempos de vida de GDI, un fallo de presentación deja de tumbar la isla y clippy queda limpio — **verificado**, ver L7 | inline | — |

T2 y T3 no se abren hasta que T1 esté verificado: son las que tocan el código de la aplicación.

**Estado:** T1–T4 verificadas. La isla vive en una ventana propia que nace fuera del tiling, y su tamaño lo decide `ui/island.slint` y nadie más. Nada commiteado (decisión del usuario, S7).

## Log

**L1** (petición verbatim del requisito):
> "vale, olvidate de engram cloud, no nos hace falta, ya veo que tienes el contexto suficiente. ahora me ha salido la island, pero se he añade como una ventana/tile en komorebi. tiene que ser flotante"

La parte de Engram Cloud se descartó por decisión del usuario en el mismo mensaje.

**L2** (verbatim):
> "vale no tengamos prisa. no existe una manera de hacer que capri nazca inekegible como pasa con yasb?"

**L3** (verbatim):
> "pues no lo se, estoy un poco perdido, que me recomendarias y porque?"

**L4** (verbatim):
> "ok dale"

**Evidencia y decisiones acumuladas**

- komorebi 0.1.41 decide en `window_is_eligible` (`komorebi/src/window.rs`): una ventana es elegible mientras tenga
  `WS_CAPTION` **y** `WS_EX_WINDOWEDGE`, y no lo es si lleva `WS_EX_DLGMODALFRAME`, `WS_EX_LAYERED` o si casa con una
  ignore rule. `WS_EX_TOOLWINDOW` no influye en la gestión (solo en el border manager).
- winit 0.30.13 fija `WS_CAPTION | WS_BORDER` y `WS_EX_WINDOWEDGE` incondicionalmente para ventanas de nivel superior
  (`to_window_styles`); solo lo evitan las ventanas hijas sin decoraciones y la pantalla completa. Slint crea la ventana
  oculta pero la muestra en la misma llamada síncrona, así que no hay hueco para parchear el estilo. Conclusión: con
  Slint + winit la ventana **no puede nacer inelegible**.
- La vía elegida para desbloquear la máquina del usuario, mientras tanto, es una regla **efímera** de komorebi:
  `komorebic ignore-rule exe capri-bar.exe` (no escribe `komorebi.json`; un reinicio de komorebi la borra).
  Verificado: la isla mide 260 × 44 y no aparece en `komorebic state` ni en `visible-windows`.
- `slint::platform::Platform` solo obliga a `create_window_adapter()`; `WindowAdapter` solo a `window()`, `size()` y
  `renderer()`. `slint::platform::software_renderer` es público y trae `MinimalSoftwareWindow` y `render_by_line`.
  Por eso T2 conserva todo Slint por encima de la capa de ventana.

**L5** (verificación de T1, `spikes/win32-window`, build con `CARGO_TARGET_DIR` dentro de `target/`):

```
estilo  pedido a CreateWindowExW: 0x90000000   (WS_POPUP | WS_VISIBLE)
exestilo pedido a CreateWindowExW: 0x00000008   (WS_EX_TOPMOST)

estilo  leido de Windows: 0x94000000
  WS_CAPTION = no   WS_POPUP = SI   WS_VISIBLE = SI
exestilo leido de Windows: 0x00000008
  WS_EX_WINDOWEDGE = no   WS_EX_TOPMOST = SI   WS_EX_APPWINDOW = no

VEREDICTO: la ventana nace SIN WS_CAPTION ni WS_EX_WINDOWEDGE -> inelegible para komorebi
ventana: 260x44 en L100 T100, visible=true
win32-spike: fin, cierro
EXIT=0
```

Con la ventana viva, y con komorebi gestionando otras tres ventanas en ese momento (la prueba no es un komorebi dormido):
`komorebic state` no incluye `win32-spike.exe`, y `komorebic visible-windows` tampoco. El proceso se cerró solo, con
código 0, sin dejar ventana residual.

Conclusión de T1: **sí se puede nacer inelegible** creando la ventana a mano. Lo que no se puede es lograrlo a través
de winit, que es por donde la crea Slint hoy.

**L6** (verificación de T2 y T3, sobre el binario real y sobre una copia renombrada):

La copia se llamó `island-test.exe` **a propósito**: la regla efímera de komorebi que había en la sesión ignoraba
`capri-bar.exe` por nombre, así que probar con ese nombre no habría demostrado nada.

```
proceso island-test  pid=20656  visible=True
clase   = capri-bar-island-window
STYLE   = 0x94000000   WS_CAPTION = no   WS_BORDER = no   WS_POPUP = SI   WS_VISIBLE = SI
EXSTYLE = 0x08080088   WS_EX_WINDOWEDGE = no   WS_EX_LAYERED = SI
                       WS_EX_TOOLWINDOW = SI   WS_EX_TOPMOST = SI   WS_EX_NOACTIVATE = SI
rect    = L830 T40 R1090 B84  = 260x44
work_area = L0 T40 R1920 B1080   -> centrado arriba: x esperado 830, medido 830; y esperado 40, medido 40

komorebi: island-test.exe NO aparece en `komorebic state` ni en `visible-windows`
          (en ese momento gestionaba otras dos ventanas, así que no estaba dormido)

captura de pantalla (276x60 = 260x44 + 8 px de margen):
  zona negra: x 8..267 (260 px), y 8..51 (44 px)
  centro (137,29) = (0,0,0) opaco
  cada esquina de la ventana coincide con los píxeles inmediatamente exteriores
    sup-izq (8,8)=(35,55,82)  fuera=[(35,55,82),(35,56,84),(35,55,82),(36,57,85)]
    inf-izq (8,51)=(17,17,27) fuera=[(17,17,27) x4]
  borde superior por el centro: ... (36,57,82) | (0,0,0) (0,0,0) ...
  borde izquierdo por el centro: (17,17,27) | (1,1,2) (0,0,0) ...  <- el (1,1,2) es el alfa parcial del radio

build: 0 errores, 0 avisos. El binario pasa de 21,2 MB a 13,4 MB y `winit` ya no aparece en el ejecutable.

T3 (sin regresión del render headless, mismos tres métodos que la integración 1):
  tamano 260x44 | centro (0,0,0,255) | esquinas (0,0,0,0) | radio 22,33 / 21,77 / 21,92 px (objetivo 22)
  `git diff -- ui/island.slint` vacío: el fichero sigue intacto (S3)
```

**L7** (verificación del retoque T4, tras el informe del verificador independiente):

```
build: 0 errores, 0 avisos       clippy --all-targets: 0 avisos, 0 errores

El tamano ya no vive en Rust. La ventana se crea con una superficie provisional de 16x16
que no llega a verse: Slint llama a `update_window_properties` antes de mostrarla, el
adapter lee `layout_constraints().preferred` y redimensiona ventana y superficie.

Prueba de que el .slint manda, en los dos sentidos:
  .slint 260x44  ->  ventana 260x44 en L830 T40 (x esperado 830)
  .slint 300x60  ->  ventana 300x60 en L810 T40 (x esperado 810)
  .slint restaurado (git diff vacio) -> ventana 260x44 otra vez

Estilos, sin cambio: STYLE 0x94000000 (WS_CAPTION no), EXSTYLE 0x08080088
  (WS_EX_WINDOWEDGE no; LAYERED + TOOLWINDOW + TOPMOST + NOACTIVATE si)

komorebi: island-test.exe ausente de `komorebic state` y de `visible-windows`
  (binario renombrado a proposito, para que la regla efimera no enmascare el resultado)

Consumo en reposo (isla quieta, 8 s, 8 procesadores logicos):
  CPU: 0,0000 s  ->  0,000 % de un nucleo  ->  0,0000 % de la maquina
  memoria de trabajo: 15,1 MB
  (el "en reposo no se nota" del manifiesto, ahora con numero)

Captura retenida como evidencia: target/verify/captura-retoque.png (276x60)
  zona negra 260x44 en (8,8), centro (0,0,0), y cada esquina igual al pixel inmediatamente exterior
```

Arreglado en T4: F3 (los objetos GDI de la superficie se restauran antes de borrarse, y todo camino de fallo
después de `CreateWindowExW` destruye la ventana), F2 (un frame que no se puede presentar se reporta una vez y
el bucle sigue), F5 (clippy limpio) y F4 (README: la integración 2 usa Win32, el crate compila solo en Windows, y
la verificación headless con `slint-viewer` sigue disponible en Linux).

**F1b (cambio de escala del monitor) queda implementado pero no ejercitado**: la comprobación está en el bucle
(compara `GetDpiForWindow` con el factor guardado y reemite `ScaleFactorChanged`), pero probarlo de verdad exige
cambiar la escala de pantalla en Windows, que es una acción del usuario.
