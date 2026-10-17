# Capri Bar — Manifiesto

La idea base de Capri Bar, expuesta como concepto. No es un plan, ni un SSD, ni un backlog: es lo que la app es y lo que no está dispuesta a ser.

**Estado:** concepto · **Fecha:** 2026-10-17

---

## 1. Qué es

Capri Bar es una **isla dinámica de escritorio**: una píldora centrada en la parte superior de la pantalla que muestra **un widget a la vez** y se amplía cuando el usuario la toca.

No es una barra de estado. Una barra de estado enseña todo lo que sabe, siempre, en una fila. Capri Bar enseña lo que importa en cada momento, en un solo sitio, y crece cuando se lo piden.

Ocupa el lugar de YASB en el escritorio —mismo papel: integración con komorebi, información del sistema, control— con otra forma y otro medio:

- **Medio:** Rust, estricto.
- **v1:** Windows, con komorebi. Linux es destino, no promesa de hoy.
- **Los dos únicos criterios de calidad:** rendimiento y fluidez de animación.

## 2. Por qué existe

**Una barra es un impuesto permanente a la atención.** Enseña todo lo que sabe para que mires una sola cosa. Y lo hace ocupando el borde de la pantalla como mobiliario: nunca desaparece, nunca se retira, nunca dice "ahora mismo no hay nada que ver".

**Y las barras que existen pesan.** YASB es Python con hojas de estilo (QSS), muchos widgets e integración con komorebi y GlazeWM. Zebar también resuelve widgets y estilos CSS, multiplataforma, sobre motores web nativos (Windows, macOS y Linux). El nicho "widgets + CSS + multiplataforma" ya está ocupado, y bien.

Lo que **no** está ocupado es la fluidez de animación nativa y el modelo de isla: enseñar una sola cosa, en el centro, y que el movimiento entre estados sea el producto. Ahí no se compite contra nadie.

Si Capri Bar vende personalización, compite de tú a tú con dos proyectos más maduros. Si vende frame y atención, no compite con nadie. Este manifiesto elige lo segundo.

## 3. Principios

1. **La isla no es una barra: es una escena.** Un widget ocupa el escenario; los demás esperan su turno. No existe la fila permanente de widgets.
2. **El rendimiento no es una característica: es la característica.** El criterio de todo es el frame. Si una idea mejora el aspecto y cuesta un frame, pierde la idea.
3. **La animación es la interfaz, no su decoración.** Aparecer, ampliarse, cambiar de widget y retirarse son transiciones, no cortes. El movimiento no se añade encima de la UI: la UI *es* el movimiento.
4. **Nada se te cuela.** Mientras el usuario está interactuando con la isla, un evento no le roba la escena. Los eventos hacen cola y esperan a que el usuario termine.
5. **Un widget es un contrato, no un dibujo.** Declara qué datos necesita, cuándo quiere la escena, qué hace ante clic y hover, y cómo se ve. Los datos están separados de la presentación: eso es lo que hará posible Linux sin reescribir nada.
6. **Personalizable en los dos ejes.** Técnico: datos, metadatos, clics, hovers. Estético: color, forma, movimiento. Ninguno de los dos se decide en el código de la app.
7. **El estilo nunca añade un runtime.** El mecanismo de estética que se elija no puede interponer entre el widget y el frame un proceso ni un motor que no controlemos. Aquí el manifiesto fija la regla y deja abierta la herramienta (§8).

## 4. Cómo se ve la isla

Tres estados, y las transiciones entre ellos:

- **Reposo.** Píldora pequeña, centrada arriba, con un solo widget. El reloj es el primer caso.
- **Ampliada.** Al pasar el ratón por encima o al hacer clic, la isla crece y ofrece la selección de widgets disponibles.
- **Ocupada.** Un evento toma la escena. Si empieza a sonar Spotify o YouTube, la isla se ocupa con la miniatura (si está disponible) y algo de metadatos, al estilo del iPhone.

## 5. Los widgets

Dos clases, y la diferencia importa:

- **Elegibles.** Son los que aparecen en la selección y pueden ocupar el reposo. Ejemplos: reloj; escritorios virtuales de komorebi, que se activa al interactuar; iconos de la bandeja de Windows; energía —apagar, reiniciar, suspender—.
- **Silenciosos.** No están en la selección: solo existen después de un evento, activo o pasivo. El reproductor de audio es el caso de referencia.

El widget que ocupa el reposo es elección del usuario, y pueden ser varios.

## 6. Qué no es

- No es una barra de estado con más widgets. No compite por cantidad.
- No es un panel permanente ni un sitio donde vivir: aparece, hace su trabajo y se retira.
- No es un proyecto de estética. Un efecto que cuesta un frame no entra, por bonito que sea.
- No es multiplataforma hoy. Linux se gana con el diseño desde el primer día, no con un parche al final.
- No es un plan. Este documento no tiene tareas, ni fases, ni fechas; las tendrá cuando deje de ser una idea.

## 7. Cómo se sabrá si funciona

La medida es por frame y por coste en reposo, y los números se fijan **antes** de escribir la primera línea de código, no después:

- **No rompe el frame bajo carga.** Música sonando, cambio de escritorios de komorebi y el ratón dentro de la isla, los tres a la vez, sin saltos.
- **En reposo no se nota.** Sin consumo medible cuando no hay nada que hacer.
- **Arranca con el escritorio sin retrasarlo.**
- **Se verifica en la máquina Windows con komorebi**, no en el equipo Debian de desarrollo, que hoy no tiene ni Rust instalado.

## 8. Puntos abiertos

Son decisiones que este documento **no** toma todavía, y las deja escritas para que no se decidan por accidente:

1. **El mecanismo de estilo.** La regla está escrita (Principio 7); la herramienta, no. Candidatos: un motor nativo con subconjunto de CSS, o un WebView con CSS real.
2. **Qué pasa exactamente con un evento que llega mientras el usuario interactúa:** si espera a que cierre o si se acumula en cola, y con qué orden de prioridad.
3. **Cómo conviven varios widgets en el reposo:** rotación, prioridad o ciclo, y si el reposo puede cambiar solo.
4. **Cómo se declara un widget:** configuración declarativa, código Rust, o las dos cosas.
5. **Una isla por monitor o una sola para todo el escritorio.**

---

Este documento cambia cuando cambia la idea, no cuando cambia el plan.
