# Capri Bar

Una **isla dinámica de escritorio**: una píldora centrada en la parte superior de la pantalla, con un widget en escena, escrita en **Rust + Slint** y enfocada a dos cosas: **rendimiento y fluidez de animación**.

La idea completa —qué es, qué no es y con qué reglas se construye— está en **[MANIFESTO.md](MANIFESTO.md)**. Este README solo explica cómo compilarlo y cómo comprobar lo que dibuja.

## Estado

**Integración 1**: la isla se dibuja (píldora negra con esquinas redondeadas). **Sin interacción, sin widgets y sin lógica.** Los widgets llegarán después, modulares.

**Integración 2**: la isla vive en una **ventana propia**, creada por el proceso, que **nace sin `WS_CAPTION` ni `WS_EX_WINDOWEDGE`** — es decir, fuera del tiling de komorebi, que solo reclama ventanas con ese par de estilos — con alfa por píxel, siempre encima, sin robar el foco y fuera del Alt+Tab. Se coloca centrada arriba en el área de trabajo del monitor principal. Sigue sin interacción. En Windows eso implica además que **este crate solo compila en Windows** (ver Requisitos).

## Requisitos

- **Rust 1.92 o superior.** Slint no compila con versiones anteriores. En esta máquina: 1.99.1.
- **En Windows**, además, las *Build Tools* de Visual Studio con la carga de trabajo de C++ (el instalador de rustup las pide al instalar el toolchain MSVC).
- **En Linux**, la isla se puede seguir **renderizando a imagen** con `slint-viewer` (ver más abajo), que es su propia herramienta. Lo que no se puede ya es compilar *este* crate: la ventana de la integración 2 se crea contra Win32 en `src/island_window.rs`. Linux sigue siendo destino, no promesa de hoy.

## Compilar y ejecutar

```
cargo run
```

Necesita un escritorio real. Abre la ventana de la isla, sin marco.

## Verificar sin pantalla (headless)

El mismo `ui/island.slint` se puede renderizar a una imagen **sin ventana y sin servidor gráfico**, que es como se verifica en un servidor o desde un agente.

Usa el visor **precompilado** de la release, no lo compiles con `cargo install slint-viewer`: esa compilación tarda muchísimo.

- Windows (x86_64): `https://github.com/slint-ui/slint/releases/download/v1.18.1/slint-viewer-x86_64-pc-windows-msvc.zip`

```
SLINT_SCALE_FACTOR=1 slint-viewer --screenshot isla.png ui/island.slint
```

Criterio de la isla, medible sobre esa imagen:

| Comprobación | Valor esperado |
|---|---|
| Tamaño | **200 × 36 px** |
| Píxel central | **#000000** opaco |
| Esquina fuera del radio | **transparente** |
| Radio medido | **18 px** |

## Estructura

| Ruta | Qué es |
|---|---|
| `ui/island.slint` | La isla: geometría y color. Es también la única fuente del tamaño de la ventana |
| `src/main.rs` | Instala la plataforma y abre la isla |
| `src/island_window.rs` | La ventana Win32: estilos, superficie de dibujo y alfa por píxel |
| `src/island_platform.rs` | La plataforma de Slint: bucle de eventos y puente con la ventana |
| `build.rs` | Compila el `.slint` durante el build (sin intérprete en runtime) |
| `MANIFESTO.md` | La idea y sus reglas |
| `.gitignore` | Ignora `target/` |

## Memoria del proyecto

La memoria del agente **no viaja dentro del repositorio**: se sincroniza con **Engram Cloud** (servidor `balneario-server` en Tailscale), en el proyecto `capri-bar`. Así el repositorio no arrastra datos de otros proyectos al historial de git.
