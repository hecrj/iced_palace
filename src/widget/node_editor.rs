use crate::core;
use crate::core::border;
use crate::core::keyboard;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Clipboard, Color, Element, Event, Length, Point, Rectangle, Shell, Size, Transformation,
    Vector, Widget,
};

use iced_widget::canvas;
use iced_widget::graphics::geometry;
use indexmap::IndexMap;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::mpsc;

pub fn node_editor<'a, T, Message, Theme, Renderer>(
    state: &'a Graph<T, Renderer>,
    view: impl Fn(Node, &'a T, Interface<'a, T, Renderer>) -> Element<'a, Message, Theme, Renderer>,
) -> NodeEditor<'a, T, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: geometry::Renderer + 'a,
{
    let nodes = state
        .nodes
        .iter()
        .map(|(id, node)| {
            view(
                *id,
                &node.state,
                Interface {
                    node,
                    sender: &state.sender,
                    interaction: &state.interaction,
                },
            )
        })
        .collect();

    NodeEditor {
        state,
        nodes,
        on_link: None,
    }
}

pub struct Interface<'a, T, Renderer = iced_widget::Renderer>
where
    Renderer: geometry::Renderer,
{
    node: &'a State<T, Renderer>,
    sender: &'a mpsc::Sender<Notification>,
    interaction: &'a Cell<Interaction>,
}

impl<'a, T> Interface<'a, T> {
    pub fn has_inputs(&self) -> bool {
        !self.node.inputs.is_empty()
    }

    pub fn has_outputs(&self) -> bool {
        !self.node.outputs.is_empty()
    }

    pub fn drag_handle<Message, Theme, Renderer>(
        &self,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_drag: impl Fn(Point) -> Message + 'a,
    ) -> Element<'a, Message, Theme, Renderer>
    where
        Message: 'a,
        Theme: 'a,
        Renderer: core::Renderer + 'a,
    {
        struct DragHandle<'a, Message, Theme, Renderer> {
            position: Point,
            content: Element<'a, Message, Theme, Renderer>,
            on_drag: Box<dyn Fn(Point) -> Message + 'a>,
        }

        struct State {
            drag_started_at: Option<(Point, Point)>,
        }

        impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
            for DragHandle<'a, Message, Theme, Renderer>
        where
            Renderer: core::Renderer,
        {
            fn tag(&self) -> tree::Tag {
                tree::Tag::of::<State>()
            }

            fn state(&self) -> tree::State {
                tree::State::new(State {
                    drag_started_at: None,
                })
            }

            fn children(&self) -> Vec<Tree> {
                vec![Tree::new(&self.content)]
            }

            fn diff(&self, tree: &mut Tree) {
                tree.diff_children(&[self.content.as_widget()]);
            }

            fn size(&self) -> Size<Length> {
                self.content.as_widget().size()
            }

            fn size_hint(&self) -> Size<Length> {
                self.content.as_widget().size_hint()
            }

            fn layout(
                &self,
                tree: &mut Tree,
                renderer: &Renderer,
                limits: &layout::Limits,
            ) -> layout::Node {
                self.content
                    .as_widget()
                    .layout(&mut tree.children[0], renderer, limits)
            }

            fn update(
                &mut self,
                tree: &mut Tree,
                event: &Event,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                _renderer: &Renderer,
                _clipboard: &mut dyn Clipboard,
                shell: &mut Shell<'_, Message>,
                _viewport: &Rectangle,
            ) {
                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                        let Some(position) = cursor.position_over(layout.bounds()) else {
                            return;
                        };

                        let state = tree.state.downcast_mut::<State>();

                        state.drag_started_at = Some((self.position, position));

                        shell.capture_event();
                    }
                    Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                        let state = tree.state.downcast_mut::<State>();

                        let Some((position, drag_started_at)) = state.drag_started_at else {
                            return;
                        };

                        let Some(current) = cursor.position() else {
                            return;
                        };

                        let translation = current - drag_started_at;

                        shell.publish((self.on_drag)(position + translation));
                    }
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                        let state = tree.state.downcast_mut::<State>();
                        state.drag_started_at = None;
                    }
                    _ => {}
                }
            }

            fn draw(
                &self,
                tree: &Tree,
                renderer: &mut Renderer,
                theme: &Theme,
                style: &renderer::Style,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                viewport: &Rectangle,
            ) {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    viewport,
                );
            }

            fn mouse_interaction(
                &self,
                tree: &Tree,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                viewport: &Rectangle,
                renderer: &Renderer,
            ) -> mouse::Interaction {
                let interaction = self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                );

                if interaction != mouse::Interaction::None {
                    return interaction;
                }

                let state = tree.state.downcast_ref::<State>();

                if state.drag_started_at.is_some() {
                    mouse::Interaction::Grabbing
                } else if cursor.is_over(layout.bounds()) {
                    mouse::Interaction::Grab
                } else {
                    mouse::Interaction::None
                }
            }
        }

        Element::new(DragHandle {
            position: self.node.position,
            content: content.into(),
            on_drag: Box::new(on_drag),
        })
    }

    pub fn inputs(&self) -> impl Iterator<Item = Connector<'a>> {
        self.node.inputs.iter().copied().map(|input| Connector {
            kind: ConnectorKind::Input(input),
            sender: self.sender,
            interaction: self.interaction,
        })
    }

    pub fn outputs(&self) -> impl Iterator<Item = Connector<'a>> {
        self.node.outputs.iter().copied().map(|output| Connector {
            kind: ConnectorKind::Output(output),
            sender: self.sender,
            interaction: self.interaction,
        })
    }
}

pub struct Connector<'a> {
    kind: ConnectorKind,
    interaction: &'a Cell<Interaction>,
    sender: &'a mpsc::Sender<Notification>,
}

#[derive(Debug, Clone, Copy)]
enum ConnectorKind {
    Input(InputId),
    Output(OutputId),
}

impl<'a> Connector<'a> {
    pub fn handle<Message, Theme, Renderer>(&self) -> Element<'a, Message, Theme, Renderer>
    where
        Renderer: core::Renderer,
    {
        struct Handle<'a> {
            kind: ConnectorKind,
            sender: &'a mpsc::Sender<Notification>,
            interaction: &'a Cell<Interaction>,
        }

        struct State {
            last_bounds: Option<Rectangle>,
            is_hovered: bool,
        }

        impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Handle<'_>
        where
            Renderer: core::Renderer,
        {
            fn tag(&self) -> tree::Tag {
                tree::Tag::of::<State>()
            }

            fn state(&self) -> tree::State {
                tree::State::new(State {
                    last_bounds: None,
                    is_hovered: false,
                })
            }

            fn size(&self) -> Size<Length> {
                Size {
                    width: Length::Fixed(10.0),
                    height: Length::Fixed(10.0),
                }
            }

            fn layout(
                &self,
                _tree: &mut Tree,
                _renderer: &Renderer,
                limits: &layout::Limits,
            ) -> layout::Node {
                let size = limits.resolve(Length::Shrink, Length::Shrink, Size::new(10.0, 10.0));

                layout::Node::new(size)
            }

            fn update(
                &mut self,
                tree: &mut Tree,
                event: &Event,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                _renderer: &Renderer,
                _clipboard: &mut dyn Clipboard,
                shell: &mut Shell<'_, Message>,
                _viewport: &Rectangle,
            ) {
                let state = tree.state.downcast_mut::<State>();

                let was_hovered = state.is_hovered;
                state.is_hovered = cursor.is_over(layout.bounds());

                if was_hovered != state.is_hovered {
                    shell.request_redraw();
                }

                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                        if cursor.is_over(layout.bounds()) =>
                    {
                        let _ = self.sender.send(Notification::ConnectorPressed(self.kind));
                    }
                    Event::Window(window::Event::RedrawRequested(_)) => {
                        let bounds = layout.bounds();

                        if state.last_bounds != Some(bounds) {
                            state.last_bounds = Some(bounds);

                            let _ = self.sender.send(match self.kind {
                                ConnectorKind::Input(input) => {
                                    Notification::InputChanged(input, bounds)
                                }
                                ConnectorKind::Output(output) => {
                                    Notification::OutputChanged(output, bounds)
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }

            fn draw(
                &self,
                _tree: &Tree,
                renderer: &mut Renderer,
                _theme: &Theme,
                _style: &renderer::Style,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                _viewport: &Rectangle,
            ) {
                let bounds = layout.bounds();

                let (color, bounds) = match self.interaction.get() {
                    Interaction::None
                    | Interaction::Panning { .. }
                    | Interaction::Resizing { .. } => (
                        Color::WHITE,
                        if cursor.is_over(layout.bounds()) {
                            bounds
                        } else {
                            bounds.shrink(2.5)
                        },
                    ),
                    Interaction::Connecting(connector) => match (connector, self.kind) {
                        (ConnectorKind::Input(_), ConnectorKind::Output(_))
                        | (ConnectorKind::Output(_), ConnectorKind::Input(_)) => (
                            Color::WHITE,
                            if cursor.is_over(layout.bounds()) {
                                bounds
                            } else {
                                bounds.shrink(2.5)
                            },
                        ),
                        _ => (Color::WHITE.scale_alpha(0.3), bounds.shrink(2.5)),
                    },
                };

                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        border: border::rounded(bounds.width),
                        ..renderer::Quad::default()
                    },
                    color,
                );
            }

            fn mouse_interaction(
                &self,
                _state: &Tree,
                layout: Layout<'_>,
                cursor: mouse::Cursor,
                _viewport: &Rectangle,
                _renderer: &Renderer,
            ) -> mouse::Interaction {
                if cursor.is_over(layout.bounds()) {
                    mouse::Interaction::Crosshair
                } else {
                    mouse::Interaction::None
                }
            }
        }

        Element::new(Handle {
            kind: self.kind,
            sender: self.sender,
            interaction: self.interaction,
        })
    }
}

pub struct NodeEditor<'a, T, Message, Theme, Renderer = iced_widget::Renderer>
where
    Renderer: geometry::Renderer,
{
    state: &'a Graph<T, Renderer>,
    nodes: Vec<Element<'a, Message, Theme, Renderer>>,
    on_link: Option<Box<dyn Fn(Link) -> Message + 'a>>,
}

impl<'a, T, Message, Theme, Renderer> NodeEditor<'a, T, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn on_link(mut self, on_link: impl Fn(Link) -> Message + 'a) -> Self {
        self.on_link = Some(Box::new(on_link));
        self
    }

    fn connection(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        state: &Internal,
        connector: ConnectorKind,
    ) -> Option<Connection> {
        let target = cursor.position()?;

        let hovered_node = self
            .state
            .nodes
            .values()
            .zip(layout.children())
            .filter_map(|(node, layout)| cursor.is_over(layout.bounds()).then_some(node))
            .next();

        let (start, end, cursor, link) = match connector {
            ConnectorKind::Input(input) => {
                let end = state.inputs.get(&input)?.center();

                let (start, link) = if let Some(node) = hovered_node {
                    let output = node
                        .outputs
                        .iter()
                        .filter_map(|output| Some((output, state.outputs.get(output)?)))
                        .min_by_key(|(_, position)| position.distance(target) as u32);

                    if let Some((output, position)) = output {
                        (
                            position.center(),
                            Some(Link {
                                input,
                                output: *output,
                            }),
                        )
                    } else {
                        (target, None)
                    }
                } else {
                    (target, None)
                };

                (start, end, start, link)
            }
            ConnectorKind::Output(output) => {
                let start = state.outputs.get(&output)?.center();

                let (end, link) = if let Some(node) = hovered_node {
                    let input = node
                        .inputs
                        .iter()
                        .filter_map(|input| Some((input, state.inputs.get(input)?)))
                        .min_by_key(|(_, position)| position.distance(target) as u32);

                    if let Some((input, position)) = input {
                        (
                            position.center(),
                            Some(Link {
                                input: *input,
                                output,
                            }),
                        )
                    } else {
                        (target, None)
                    }
                } else {
                    (target, None)
                };

                (start, end, end, link)
            }
        };

        Some(Connection {
            start,
            end,
            cursor: mouse::Cursor::Available(cursor),
            link,
        })
    }
}

struct Connection {
    start: Point,
    end: Point,
    cursor: mouse::Cursor,
    link: Option<Link>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    input: InputId,
    output: OutputId,
}

impl<T> From<(Output<T>, Input<T>)> for Link {
    fn from((output, input): (Output<T>, Input<T>)) -> Self {
        Self {
            input: input.id,
            output: output.id,
        }
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for NodeEditor<'a, T, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Internal>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Internal {
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            modifiers: keyboard::Modifiers::default(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.nodes.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.nodes);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let nodes = self
            .nodes
            .iter()
            .zip(&mut tree.children)
            .zip(self.state.nodes.values())
            .map(|((node, tree), state)| {
                let size = node.as_widget().size_hint();
                let limits = state.size.get();

                node.as_widget()
                    .layout(
                        tree,
                        renderer,
                        &layout::Limits::new(
                            Size::ZERO,
                            Size::new(
                                if size.width.is_fill() {
                                    limits.width
                                } else {
                                    f32::INFINITY
                                },
                                if size.height.is_fill() {
                                    limits.height
                                } else {
                                    f32::INFINITY
                                },
                            ),
                        ),
                    )
                    .move_to(state.position)
            })
            .collect();

        layout::Node::with_children(limits.max(), nodes)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let cursor_external = cursor;
        let cursor = if cursor.is_over(layout.bounds()) {
            cursor * self.state.transformation().inverse()
        } else {
            cursor.levitate()
        };
        let mut cursor_node = cursor;

        for node in self.state.order.borrow().iter().rev() {
            if shell.is_event_captured() {
                break;
            }

            let Some(index) = self.state.nodes.get_index_of(node) else {
                continue;
            };

            let node = &mut self.nodes[index];
            let layout = layout.child(index);
            let tree = &mut tree.children[index];

            node.as_widget_mut().update(
                tree,
                event,
                layout,
                cursor_node,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if cursor_node.is_over(layout.bounds()) {
                cursor_node = mouse::Cursor::Unavailable;
            }
        }

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            let state = tree.state.downcast_mut::<Internal>();

            state.modifiers = *modifiers;
            return;
        }

        if let Event::Window(window::Event::RedrawRequested(_))
        | Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
        {
            let state = tree.state.downcast_mut::<Internal>();

            for notification in self.state.receiver.try_iter() {
                match notification {
                    Notification::InputChanged(input, bounds) => {
                        let _ = state.inputs.insert(input, bounds);

                        if let Some(node) = self.state.nodes.get(&input.node) {
                            node.links.clear();
                        }
                    }
                    Notification::OutputChanged(output, bounds) => {
                        let _ = state.outputs.insert(output, bounds);

                        for (input, _) in self
                            .state
                            .links
                            .iter()
                            .filter(|(_input, candidate)| **candidate == output)
                        {
                            if let Some(node) = self.state.nodes.get(&input.node) {
                                node.links.clear();
                            }
                        }
                    }
                    Notification::ConnectorPressed(connector) => {
                        if self.on_link.is_some() {
                            self.state
                                .interaction
                                .set(Interaction::Connecting(connector));

                            shell.request_redraw();
                        }
                    }
                }
            }
        }

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let state = tree.state.downcast_mut::<Internal>();

            if state.modifiers.command() {
                match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        let zoom = self.state.zoom.get();
                        let factor = if *y > 0.0 { 2.0 } else { 0.5 };
                        let new_zoom = (zoom * factor).clamp(0.25, 4.0);

                        self.state.zoom.set(new_zoom);

                        if let Some(pointer) = cursor.position() {
                            self.state.translation.set(
                                self.state.translation.get()
                                    + (Point::ORIGIN - pointer) * (new_zoom - zoom),
                            );
                        }

                        shell.request_redraw();
                    }
                }
            } else {
                let panning = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => {
                        let pixels = 100.0 * y;

                        if state.modifiers.shift() {
                            Vector::new(pixels, 0.0)
                        } else {
                            Vector::new(0.0, pixels)
                        }
                    }
                    mouse::ScrollDelta::Pixels { x, y } => Vector::new(*x, *y),
                };

                self.state
                    .translation
                    .set(self.state.translation.get() + panning);

                shell.request_redraw();
            }
        }

        match self.state.interaction.get() {
            Interaction::None => {
                if let Event::Mouse(mouse::Event::ButtonPressed(
                    button @ (mouse::Button::Left | mouse::Button::Middle),
                )) = event
                {
                    let Some(from) = cursor_external.position() else {
                        return;
                    };

                    if *button == mouse::Button::Left {
                        let mut order = self.state.order.borrow_mut();
                        let node_hovered = order.iter().enumerate().rev().find_map(|(i, node)| {
                            let node_index = self.state.nodes.get_index_of(node)?;

                            cursor
                                .is_over(layout.child(node_index).bounds())
                                .then_some((i, node_index))
                        });

                        if let Some((order_index, node_index)) = node_hovered {
                            let node = order.remove(order_index);
                            order.push(node);

                            let size_hint = self.nodes[node_index].as_widget().size_hint();

                            let Some(from) = cursor.position() else {
                                return;
                            };

                            if let Some(resize_direction) = Direction::detect(
                                size_hint,
                                layout.child(node_index).bounds(),
                                from,
                            ) {
                                let Some(node) = self.state.nodes.get(&node) else {
                                    return;
                                };

                                self.state.interaction.set(Interaction::Resizing {
                                    node: node.id,
                                    original: node.size.get(),
                                    direction: resize_direction,
                                    from,
                                });
                            }

                            shell.request_redraw();
                            return;
                        }
                    }

                    if shell.is_event_captured() {
                        return;
                    }

                    self.state
                        .interaction
                        .set(Interaction::Panning { from, to: from });
                    shell.request_redraw();
                }
            }
            Interaction::Connecting(connector) => {
                if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
                    let state = tree.state.downcast_ref::<Internal>();

                    if let Some(on_link) = &self.on_link
                        && let Some(connection) = self.connection(layout, cursor, state, connector)
                        && let Some(link) = connection.link
                    {
                        shell.publish(on_link(link));
                    }

                    self.state.interaction.set(Interaction::None);
                }
            }
            Interaction::Panning { from, to } => {
                if let Some(to) = cursor_external.position() {
                    self.state
                        .interaction
                        .set(Interaction::Panning { from, to });
                }

                if let Event::Mouse(mouse::Event::ButtonReleased(
                    mouse::Button::Left | mouse::Button::Middle,
                )) = event
                {
                    self.state.interaction.set(Interaction::None);

                    self.state
                        .translation
                        .set(self.state.translation.get() + (to - from));

                    shell.request_redraw();
                    return;
                }
            }
            Interaction::Resizing {
                node,
                original,
                direction,
                from,
            } => {
                if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
                    self.state.interaction.set(Interaction::None);
                } else if let Some(to) = cursor.position()
                    && let Some(node) = self.state.nodes.get(&node)
                {
                    let old_size = node.size.get();
                    let new_width = (original.width + to.x - from.x).round().max(50.0);
                    let new_height = (original.height + to.y - from.y).round().max(10.0);

                    match direction {
                        Direction::Horizontal => {
                            node.size.set(Size {
                                width: new_width,
                                ..original
                            });
                        }
                        Direction::Vertical => {
                            node.size.set(Size {
                                height: new_height,
                                ..original
                            });
                        }
                        Direction::Diagonal => {
                            node.size.set(Size {
                                width: new_width,
                                height: new_height,
                            });
                        }
                    }

                    if old_size != node.size.get() {
                        shell.invalidate_layout();
                        shell.request_redraw();
                    }
                }
            }
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event
            && let Interaction::Connecting(_) | Interaction::Panning { .. } =
                self.state.interaction.get()
        {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<Internal>();

        let bounds = layout.bounds();
        let transformation = self.state.transformation();
        let inverse = transformation.inverse();

        let mut cursor = cursor * transformation.inverse();
        let viewport = bounds.intersection(viewport).unwrap_or(*viewport) * inverse;

        renderer.start_transformation(transformation);
        renderer.start_layer(viewport);

        for node in self.state.nodes.values() {
            let geometry = node
                .links
                .draw_with_bounds(renderer, Rectangle::INFINITE, |frame| {
                    for input in &node.inputs {
                        let Some(output) = self.state.links.get(input) else {
                            continue;
                        };

                        let Some(start_bounds) = state.outputs.get(output) else {
                            continue;
                        };

                        let Some(end_bounds) = state.inputs.get(input) else {
                            continue;
                        };

                        let start = start_bounds.center();
                        let end = end_bounds.center();

                        draw_connection(frame, start, end);
                    }
                });

            renderer.draw_geometry(geometry);
        }

        match self.state.interaction.get() {
            Interaction::None | Interaction::Panning { .. } | Interaction::Resizing { .. } => {}
            Interaction::Connecting(connector) => {
                let connection = self.connection(layout, cursor, state, connector);

                if let Some(connection) = connection {
                    let mut frame = canvas::Frame::with_bounds(renderer, Rectangle::INFINITE);

                    draw_connection(&mut frame, connection.start, connection.end);

                    renderer.draw_geometry(frame.into_geometry());

                    cursor = connection.cursor;
                }
            }
        }

        renderer.end_layer();
        renderer.start_layer(viewport);

        let mut bounds: Vec<Rectangle> = Vec::with_capacity(self.state.nodes.len());

        for node in self.state.order.borrow().iter() {
            let Some(index) = self.state.nodes.get_index_of(node) else {
                continue;
            };

            let node = &self.nodes[index];
            let layout = layout.child(index);
            let tree = &tree.children[index];

            if let Some(clip_bounds) = viewport.intersection(&layout.bounds()) {
                if bounds
                    .iter()
                    .any(|previous| previous.intersects(&clip_bounds))
                {
                    renderer.end_layer();
                    renderer.start_layer(viewport);
                }

                node.as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, &viewport);

                bounds.push(clip_bounds);
            }
        }

        renderer.end_layer();
        renderer.end_transformation();
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        match self.state.interaction.get() {
            Interaction::None => {}
            Interaction::Connecting(_) => return mouse::Interaction::Crosshair,
            Interaction::Panning { .. } => return mouse::Interaction::Grabbing,
            Interaction::Resizing { direction, .. } => return direction.to_mouse_interaction(),
        }

        let cursor = if cursor.is_over(layout.bounds()) {
            cursor * self.state.transformation().inverse()
        } else {
            cursor.levitate()
        };

        for node in self.state.order.borrow().iter().rev() {
            let Some(index) = self.state.nodes.get_index_of(node) else {
                continue;
            };

            let node = &self.nodes[index];
            let layout = layout.child(index);
            let tree = &tree.children[index];

            let interaction = node
                .as_widget()
                .mouse_interaction(tree, layout, cursor, viewport, renderer);

            if interaction != mouse::Interaction::None {
                return interaction;
            }

            if let Some(position) = cursor.position_over(layout.bounds()) {
                return Direction::detect(node.as_widget().size_hint(), layout.bounds(), position)
                    .map(Direction::to_mouse_interaction)
                    .unwrap_or(interaction);
            }
        }

        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let transformation = self.state.transformation();
        let panning = transformation.translation() * (1.0 / transformation.scale_factor());

        let content = overlay::from_children(
            &mut self.nodes,
            tree,
            layout,
            renderer,
            viewport,
            translation + panning,
        )?;

        Some(overlay::Element::new(Box::new(Overlay {
            content,
            transformation: transformation * Transformation::translate(-panning.x, -panning.y),
        })))
    }
}

struct Overlay<'a, Message, Theme, Renderer> {
    content: overlay::Element<'a, Message, Theme, Renderer>,
    transformation: Transformation,
}

impl<'a, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'a, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.content.as_overlay_mut().layout(
            renderer,
            bounds * (1.0 / self.transformation.scale_factor()),
        )
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        self.content.as_overlay_mut().update(
            event,
            layout,
            cursor * self.transformation.inverse(),
            renderer,
            clipboard,
            shell,
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        renderer.start_transformation(self.transformation);

        self.content.as_overlay().draw(
            renderer,
            theme,
            style,
            layout,
            cursor * self.transformation.inverse(),
        );

        renderer.end_transformation();
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_overlay().mouse_interaction(
            layout,
            cursor * self.transformation.inverse(),
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let content = self.content.as_overlay_mut().overlay(layout, renderer)?;

        Some(overlay::Element::new(Box::new(Overlay {
            content,
            transformation: self.transformation,
        })))
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced_core::widget::Operation,
    ) {
        self.content
            .as_overlay_mut()
            .operate(layout, renderer, operation);
    }

    fn index(&self) -> f32 {
        self.content.as_overlay().index()
    }
}

impl<'a, T, Message, Theme, Renderer> From<NodeEditor<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: geometry::Renderer + 'static,
{
    fn from(node_editor: NodeEditor<'a, T, Message, Theme, Renderer>) -> Self {
        Element::new(node_editor)
    }
}

#[derive(Debug)]
pub struct Input<T> {
    id: InputId,
    _type: PhantomData<T>,
}

#[derive(Debug)]
pub struct Output<T> {
    id: OutputId,
    _type: PhantomData<T>,
}

impl<T> Copy for Input<T> {}

impl<T> Clone for Input<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Clone for Output<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Output<T> {}

pub struct Value(Option<Rc<dyn Any>>);

impl Value {
    fn new<T>(f: impl FnOnce() -> Option<T>) -> Self
    where
        T: 'static,
    {
        Self(f().map(|v| Rc::new(v) as _))
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct Graph<T, Renderer = iced_widget::Renderer>
where
    Renderer: geometry::Renderer,
{
    nodes: IndexMap<Node, State<T, Renderer>>,
    links: HashMap<InputId, OutputId>,
    values: HashMap<OutputId, Value>,
    current: u64,
    evaluate: fn(&mut T, Data<'_>),
    sender: mpsc::Sender<Notification>,
    receiver: mpsc::Receiver<Notification>,
    interaction: Cell<Interaction>,
    translation: Cell<Vector>,
    zoom: Cell<f32>,
    order: RefCell<Vec<Node>>,
}

pub struct Data<'a> {
    links: &'a HashMap<InputId, OutputId>,
    values: &'a mut HashMap<OutputId, Value>,
}

impl Data<'_> {
    pub fn get<T>(&self, input: &Input<T>) -> Option<&T>
    where
        T: 'static,
    {
        let output_id = self.links.get(&input.id)?;
        let output = self.values.get(output_id)?;

        output.0.as_ref()?.downcast_ref()
    }

    pub fn set<T>(&mut self, output: &Output<T>, value: T)
    where
        T: 'static,
    {
        self.set_with(output, |_| Some(value));
    }

    pub fn set_with<T>(&mut self, output: &Output<T>, f: impl FnOnce(&Self) -> Option<T>)
    where
        T: 'static,
    {
        let _ = self.values.insert(output.id, Value::new(|| f(self)));
    }
}

struct Internal {
    inputs: HashMap<InputId, Rectangle>,
    outputs: HashMap<OutputId, Rectangle>,
    modifiers: keyboard::Modifiers,
}

#[derive(Debug, Clone, Copy)]
enum Interaction {
    None,
    Connecting(ConnectorKind),
    Panning {
        from: Point,
        to: Point,
    },
    Resizing {
        node: Node,
        original: Size,
        direction: Direction,
        from: Point,
    },
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Horizontal,
    Vertical,
    Diagonal,
}

impl Direction {
    fn detect(size: Size<Length>, bounds: Rectangle, cursor: Point) -> Option<Self> {
        let horizontal = size.width.is_fill() && cursor.x >= bounds.x + bounds.width - 5.0;
        let vertical = size.height.is_fill() && cursor.y >= bounds.y + bounds.height - 5.0;

        Some(match (horizontal, vertical) {
            (false, false) => None?,
            (false, true) => Self::Vertical,
            (true, false) => Self::Horizontal,
            (true, true) => Self::Diagonal,
        })
    }

    fn to_mouse_interaction(self) -> mouse::Interaction {
        match self {
            Self::Horizontal => mouse::Interaction::ResizingHorizontally,
            Self::Vertical => mouse::Interaction::ResizingVertically,
            Self::Diagonal => mouse::Interaction::ResizingDiagonallyDown,
        }
    }
}

#[derive(Debug)]
enum Notification {
    InputChanged(InputId, Rectangle),
    OutputChanged(OutputId, Rectangle),
    ConnectorPressed(ConnectorKind),
}

struct State<T, Renderer>
where
    Renderer: geometry::Renderer,
{
    id: Node,
    state: T,
    position: Point,
    size: Cell<Size>,
    inputs: Vec<InputId>,
    outputs: Vec<OutputId>,
    links: canvas::Cache<Renderer>,
}

impl<T, Renderer> State<T, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn is_root(&self, links: &HashMap<InputId, OutputId>) -> bool {
        self.inputs.iter().all(|input| !links.contains_key(input))
    }
}

impl<T, Renderer> Clone for State<T, Renderer>
where
    T: Clone,
    Renderer: geometry::Renderer,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            state: self.state.clone(),
            position: self.position,
            size: self.size.clone(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            links: canvas::Cache::new(),
        }
    }
}

impl<T, Renderer> Graph<T, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn new(evaluate: fn(&mut T, Data<'_>)) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            nodes: IndexMap::new(),
            links: HashMap::new(),
            values: HashMap::new(),
            current: 0,
            evaluate,
            sender,
            receiver,
            interaction: Cell::new(Interaction::None),
            translation: Cell::new(Vector::ZERO),
            zoom: Cell::new(1.0),
            order: RefCell::default(),
        }
    }

    pub fn get(&self, node: Node) -> Option<&T> {
        self.nodes.get(&node).map(|node| &node.state)
    }

    pub fn update<O>(&mut self, node: Node, f: impl FnOnce(&mut T, Data<'_>) -> O) -> O
    where
        O: Default,
    {
        let result = {
            let Some(node) = self.nodes.get_mut(&node) else {
                return O::default();
            };

            let data = Data {
                links: &self.links,
                values: &mut self.values,
            };

            f(&mut node.state, data)
        };

        self.invalidate(node);

        result
    }

    pub fn input<A>(&self, input: &Input<A>) -> Option<&A>
    where
        A: 'static,
    {
        let output_id = self.links.get(&input.id)?;
        let output = self.values.get(output_id)?;

        output.0.as_ref()?.downcast_ref()
    }

    pub fn build(&mut self) -> Builder<'_, T, Renderer> {
        let node = Node(self.current);
        self.current += 1;

        Builder {
            graph: self,
            node,
            size: Size::new(200.0, 200.0), // TODO: Configurable!
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn push(
        &mut self,
        position: impl Into<Point>,
        f: impl FnOnce(&mut Builder<'_, T, Renderer>) -> T,
    ) {
        let mut builder = self.build();
        let state = f(&mut builder);
        builder.finish(position, state);
    }

    pub fn link(&mut self, link: impl Into<Link>) {
        let link = link.into();
        self.links.insert(link.input, link.output);

        if let Some(node) = self.nodes.get(&link.input.node) {
            node.links.clear();
        }

        self.invalidate(link.input.node);
    }

    pub fn move_to(&mut self, node: Node, position: impl Into<Point>) {
        if let Some(node) = self.nodes.get_mut(&node) {
            let position = position.into();

            node.position = Point::new(position.x.round(), position.y.round());
        }
    }

    pub fn dependencies(&self, node: Node) -> HashSet<Node> {
        let mut dependencies = HashSet::new();
        let mut pending = VecDeque::new();

        pending.push_back(node);

        while let Some(current) = pending.pop_front() {
            let Some(current) = self.nodes.get(&current) else {
                continue;
            };

            for input in &current.inputs {
                let Some(output) = self.links.get(input) else {
                    continue;
                };

                if !dependencies.contains(&output.node) && !pending.contains(&output.node) {
                    dependencies.insert(output.node);
                    pending.push_back(output.node);
                }
            }
        }

        dependencies
    }

    pub fn schedule(&self, node: Node) -> Vec<Node> {
        let mut schedule = Vec::new();
        let mut links = self.links.clone();

        let dependencies = self.dependencies(node);
        let mut pending: Vec<_> = dependencies
            .iter()
            .filter_map(|dependency| {
                self.nodes
                    .get(dependency)?
                    .is_root(&links)
                    .then_some(dependency)
            })
            .collect();

        while let Some(current) = pending.pop() {
            let Some(current_node) = self.nodes.get(current) else {
                continue;
            };

            schedule.push(*current);

            for candidate in &dependencies {
                let Some(candidate_node) = self.nodes.get(candidate) else {
                    continue;
                };

                let connection = candidate_node.inputs.iter().find(|input| {
                    links
                        .get(input)
                        .is_some_and(|output| current_node.outputs.contains(output))
                });

                if let Some(connection) = connection {
                    let _ = links.remove(connection);

                    if candidate_node.is_root(&links) {
                        pending.push(candidate);
                    }
                }
            }
        }

        schedule.push(node);
        schedule
    }

    fn transformation(&self) -> Transformation {
        let translation = match self.interaction.get() {
            Interaction::None | Interaction::Connecting(_) | Interaction::Resizing { .. } => {
                self.translation.get()
            }
            Interaction::Panning { from, to } => self.translation.get() + (to - from),
        };

        Transformation::translate(translation.x.round(), translation.y.round())
            * Transformation::scale(self.zoom.get())
    }

    fn evaluate(&mut self, node: Node) {
        let Some(node) = self.nodes.get_mut(&node) else {
            return;
        };

        for output in &node.outputs {
            let _ = self.values.remove(output);
        }

        let data = Data {
            links: &self.links,
            values: &mut self.values,
        };

        (self.evaluate)(&mut node.state, data);
    }

    fn invalidate(&mut self, node: Node) {
        let mut pending = VecDeque::new();
        let mut visited = HashSet::new();

        pending.push_back(node);

        while let Some(node) = pending.pop_front() {
            self.evaluate(node);

            let dependants = self
                .links
                .iter()
                .filter(|(_, output)| output.node == node)
                .map(|(input, _)| input);

            for dependant in dependants {
                if !visited.contains(&dependant.node) && !pending.contains(&dependant.node) {
                    pending.push_back(dependant.node);
                }
            }

            visited.insert(node);
        }
    }
}

impl<T, Renderer> Clone for Graph<T, Renderer>
where
    T: Clone,
    Renderer: geometry::Renderer,
{
    fn clone(&self) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            nodes: self.nodes.clone(),
            links: self.links.clone(),
            values: self.values.clone(),
            current: self.current,
            evaluate: self.evaluate,
            sender,
            receiver,
            interaction: Cell::new(Interaction::None),
            translation: self.translation.clone(),
            zoom: self.zoom.clone(),
            order: self.order.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InputId {
    node: Node,
    name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputId {
    node: Node,
    name: &'static str,
}

pub struct Builder<'a, T, Renderer = iced_widget::Renderer>
where
    Renderer: geometry::Renderer,
{
    graph: &'a mut Graph<T, Renderer>,
    node: Node,
    size: Size,
    inputs: Vec<InputId>,
    outputs: Vec<OutputId>,
}

impl<T, Renderer> Builder<'_, T, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn input<A>(&mut self, name: &'static str) -> Input<A> {
        let id = InputId {
            node: self.node,
            name,
        };

        self.inputs.push(id);

        Input {
            id,
            _type: PhantomData,
        }
    }

    pub fn output<A>(&mut self, name: &'static str) -> Output<A> {
        let id = OutputId {
            node: self.node,
            name,
        };

        self.outputs.push(id);

        Output {
            id,
            _type: PhantomData,
        }
    }

    pub fn size(&mut self, size: impl Into<Size>) {
        self.size = size.into();
    }

    pub fn finish(self, position: impl Into<Point>, state: T) {
        let _ = self.graph.nodes.insert(
            self.node,
            State {
                id: self.node,
                state,
                position: position.into(),
                size: Cell::new(self.size),
                inputs: self.inputs,
                outputs: self.outputs,
                links: canvas::Cache::new(),
            },
        );

        self.graph.order.get_mut().push(self.node);
        self.graph.invalidate(self.node);
    }
}

fn draw_connection<Renderer>(frame: &mut canvas::Frame<Renderer>, start: Point, end: Point)
where
    Renderer: geometry::Renderer,
{
    let line = {
        let mut builder = canvas::path::Builder::new();

        builder.move_to(start);
        builder.bezier_curve_to(
            start + Vector::new(100.0, 0.0),
            end + Vector::new(-100.0, 0.0),
            end,
        );

        builder.build()
    };

    frame.stroke(
        &line,
        canvas::Stroke {
            style: canvas::Style::Solid(Color::WHITE),
            width: 3.0,
            ..canvas::Stroke::default()
        },
    );
}
