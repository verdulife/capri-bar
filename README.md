# Capri Bar

Una **isla dinámica de escritorio**: una píldora centrada en la parte superior de la pantalla, con un widget en escena, escrita en **Rust + Slint** y enfocada a dos cosas: **rendimiento y fluidez de animación**.

La idea completa —qué es, qué no es y con qué reglas se construye— está en **[MANIFESTO.md](MANIFESTO.md)**. Este README solo explica cómo compilarlo y cómo comprobar lo que dibuja.

## Estado

**Integración 1**: la isla se dibuja (píldora negra con esquinas redondeadas). **Sin interacción, sin widgets y sin lógica.** Los widgets llegarán después, modulares.

## Requisitos

- **Rust 1.92 o superior.** Slint no compila con versiones anteriores. En esta máquina: 1.99.1.
- **En Windows**, además, las *Build Tools* de Visual Studio con la carga de trabajo de C++ (el instalador de rustup las pide al instalar el toolchain MSVC).
- **En Linux**, para *ejecutar la ventana en pantalla* hacen falta X11 o Wayland y una pila OpenGL (Mesa). Para **compilar y renderizar a imagen** no hace falta nada de eso.

## Compilar y ejecutar

```
cargo run
```

Necesita un escritorio real. Abre la ventana de la isla, sin marco.

## Verificar sin pantalla (headless)

El mismo `ui/island.slint` se puede renderizar a una imagen **sin ventana y sin servidor gráfico**, que es como se verifica en un servidor o desde un agente:

```
cargo install slint-viewer
SLINT_SCALE_FACTOR=1 slint-viewer --screenshot isla.png ui/island.slint
```

Criterio de la integración 1, medible sobre esa imagen:

| Comprobación | Valor esperado |
|---|---|
| Tamaño | **260 × 44 px** |
| Píxel central | **#000000** opaco |
| Esquina fuera del radio | **transparente** |
| Radio medido | **22 px** |

## Estructura

| Ruta | Qué es |
|---|---|
| `ui/island.slint` | La isla: geometría y color |
| `src/main.rs` | Abre la ventana y nada más |
| `build.rs` | Compila el `.slint` durante el build (sin intérprete en runtime) |
| `MANIFESTO.md` | La idea y sus reglas |
| `.gitignore` | Ignora `target/` |

## Memoria del proyecto

La memoria del agente **no viaja dentro del repositorio**: se sincroniza con **Engram Cloud** (servidor `balneario-server` en Tailscale), en el proyecto `capri-bar`. Así el repositorio no arrastra datos de otros proyectos al historial de git.
