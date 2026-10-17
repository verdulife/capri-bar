# Integración 3 — El primer widget: el reloj

La isla deja de ser un dibujo: en reposo enseña la hora. Es el primer widget y el primer texto
que se dibuja, así que además pone a prueba el camino de temporizadores y el de fuentes del
renderizador software, que son los que después necesitarán la interacción y las animaciones.

## Specs

**S1.** En reposo, la isla enseña un reloj dentro de la píldora, en formato **HH:MM**, y se
   actualiza **al cambiar el minuto**, no en cada frame.
> "que tal si añadimos el primer widget de reloj?"
> Formato elegido por el usuario: "HH:MM, y despierta al cambiar el minuto (recomendado)"

**S2.** El widget declara sus datos y la presentación va aparte (principio 5 del manifiesto):
   `ui/widgets/clock.slint` recibe la hora como dato (`property <string> time`) y **los datos los
   pone Rust**. El widget no lee el reloj del sistema ni se lo dibuja por su cuenta.

**S3.** **Sin dependencias nuevas**: la hora se lee con `GetLocalTime` de Windows, que ya está
   disponible por `windows-sys`. Cuando Linux deje de ser destino y sea objetivo, se cambia esa
   única función por una fuente portable.

**S4.** La isla solo se redibuja cuando cambia lo que se muestra. Criterio medible: el consumo en
   reposo sigue siendo despreciable (hoy: 0,0000 s de CPU en 8 s) y en un minuto hay **una sola
   activación** del bucle, no sesenta.

**S5.** Criterio de medida de la isla, actualizado para cuando hay texto:
> Píldora de **200 × 36 px** con radio **18 px**; esquinas fuera del radio **transparentes**; el
> fondo de la píldora **#000000** en un punto fuera del texto; y **hay tinta de texto** dentro.

   Ojo: el criterio anterior ("el píxel central es #000000") **deja de ser válido** en cuanto hay
   texto centrado, porque el centro puede caer dentro de una letra o entre dos. Por eso el fondo se
   comprueba en un punto alejado del texto y la presencia de texto se comprueba aparte.

**S6.** `ui/island.slint` gana el widget y, con él, deja de ser "geometría y color": es un cruce
   deliberado de la regla de la integración 1 ("SIN interacción ni lógica"), pedido por el usuario.

**S7.** Nada se commitea ni se publica sin decisión del usuario.

## Tasks

| ID | Specs | Tarea | Ruta | Commit |
|---|---|---|---|---|
| T1 | S5 | Comprobar **antes de nada** que el renderizador software dibuja texto en esta máquina (si no encuentra fuente, el reloj saldría en blanco) — **verificado**, ver L2 | inline | — |
| T2 | S1, S2, S5, S6 | `ui/widgets/clock.slint` con la hora como dato y un valor por defecto para el render headless; la isla lo coloca centrado en la píldora — **verificado**, ver L2 | inline | — |
| T3 | S1, S3, S4 | El reloj en Rust: leer la hora, formatearla y despertar la isla al cambiar el minuto — **verificado**, ver L2 | inline | — |
| T4 | S4, S5 | Verificación: criterio de medida nuevo, dos capturas separadas por un minuto distinto (deben diferir) y dos dentro del mismo minuto (deben ser idénticas), consumo en reposo, komorebi y README — **verificado**, ver L2 | inline | — |

## Log

**L1** (petición verbatim del requisito, y la decisión de formato):

> "que tal si añadimos el primer widget de reloj?"

> "HH:MM, y despierta al cambiar el minuto (recomendado)"

Contexto de la decisión: la integración 3 se planteó primero como traducción de entrada para el estado
ampliado; el usuario propuso empezar por el reloj, que es el primer caso que nombra el manifiesto
("Reposo. Píldora pequeña, centrada arriba, con un solo widget. El reloj es el primer caso"). La
interacción y el estado ampliado quedan para después, con sus propias decisiones (modelo de crecimiento
de la ventana: redimensionar solo en los cambios de estado y animar dentro).

**L2** (verificación de T1–T4):

```
T1  el renderizador software dibuja texto: sonda desechable en target/verify/text-probe.slint
    255 px de tinta, caja de 48x15 px para "12:34" a 20 px, centrada en 60,5 contra 60,0,
    y 13 colores distintos en la tinta (antialiasing de glifos de verdad)

T2  el dato manda sobre el dibujo (sin la app, con slint-viewer --load-data):
    mismo dato  -> render byte a byte identico (md5 c9a4d2aec9a6)
    otro dato   -> tinta distinta (181 px para 07:05 contra 179 px para 23:59)

T3  la logica del reloj esta fijada por los dos primeros tests automatizados del proyecto:
    cargo test -> 2 passed (relleno de ceros; tiempo hasta el cambio de minuto, incluido 59,999 s)

T4  criterio headless (criterio-headless.py sobre isla-reloj.png):
    200x36 | esquinas transparentes | fondo #000000 en (6,18) | 220 px de tinta | radio 18,04  -> CUMPLIDO
    criterio en pantalla (captura.py sobre reloj-A.png):
    200x36 | esquinas mostrando el escritorio | fondo #000000 en (14,26) | 124 px de tinta -> CUMPLIDO
    el minuto: dos capturas dentro del mismo minuto son identicas (md5 ab32c459e08b); otra despues
    del cambio de minuto es distinta (77ee4da53709)
    la hora ensenada es la del sistema: caja de tinta en pantalla 38x11 px contra 40x12 px del render
    headless de esa misma hora (misma geometria de texto dentro de +-2 px)
    consumo en reposo: 75 s con un cambio de minuto incluido -> 0,0000 s de CPU medidos, que es menos
    de lo que Windows sabe informar (resolucion de ~15,6 ms), y 22,8 MB de memoria de trabajo
    el criterio del README quedo actualizado: el "pixel central #000000" ya no vale con el reloj
    centrado, y en su lugar se mide el fondo fuera del texto y la presencia de tinta
```
