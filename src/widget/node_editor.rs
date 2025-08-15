use crate::core;
use crate::core::border;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Clipboard, Color, Element, Event, Length, Point, Rectangle, Shell, Size, Vector, Widget,
};

use iced_widget::canvas;
use iced_widget::graphics::geometry;

use std::any::Any;
use std::cell::Cell;
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

                if cursor.is_over(layout.bounds()) {
                    let state = tree.state.downcast_ref::<State>();

                    if state.drag_started_at.is_some() {
                        mouse::Interaction::Grabbing
                    } else {
                        mouse::Interaction::Grab
                    }
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
                    Interaction::None => (
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
        let target = cursor.position_in(layout.bounds())?;

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
                node.as_widget()
                    .layout(
                        tree,
                        renderer,
                        &layout::Limits::new(Size::ZERO, Size::INFINITY),
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
        for ((node, layout), tree) in self
            .nodes
            .iter_mut()
            .zip(layout.children())
            .zip(&mut tree.children)
        {
            node.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }

        match event {
            Event::Window(window::Event::RedrawRequested(_))
            | Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
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
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                match self.state.interaction.get() {
                    Interaction::None => {}
                    Interaction::Connecting(connector) => {
                        let state = tree.state.downcast_ref::<Internal>();

                        if let Some(on_link) = &self.on_link
                            && let Some(connection) =
                                self.connection(layout, cursor, state, connector)
                            && let Some(link) = connection.link
                        {
                            shell.publish(on_link(link));
                        }
                    }
                }

                self.state.interaction.set(Interaction::None);
            }
            _ => {}
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event
            && let Interaction::Connecting(_) = self.state.interaction.get()
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
        mut cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<Internal>();

        for node in self.state.nodes.values() {
            let geometry = node.links.draw(renderer, Size::INFINITY, |frame| {
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
            Interaction::None => {}
            Interaction::Connecting(connector) => {
                let connection = self.connection(layout, cursor, state, connector);

                if let Some(connection) = connection {
                    let mut frame = canvas::Frame::new(renderer, Size::INFINITY);

                    draw_connection(&mut frame, connection.start, connection.end);

                    renderer.draw_geometry(frame.into_geometry());

                    cursor = connection.cursor;
                }
            }
        }

        renderer.with_layer(*viewport, |renderer| {
            for ((node, layout), tree) in
                self.nodes.iter().zip(layout.children()).zip(&tree.children)
            {
                node.as_widget()
                    .draw(tree, renderer, theme, style, layout, cursor, viewport);
            }
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let interaction = self
            .nodes
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(state, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default();

        if interaction != mouse::Interaction::None {
            return interaction;
        }

        match self.state.interaction.get() {
            Interaction::None => mouse::Interaction::None,
            Interaction::Connecting(_) => mouse::Interaction::Crosshair,
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.nodes,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
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
    nodes: HashMap<Node, State<T, Renderer>>,
    links: HashMap<InputId, OutputId>,
    values: HashMap<OutputId, Value>,
    current: u64,
    evaluate: fn(&mut T, Data<'_>),
    sender: mpsc::Sender<Notification>,
    receiver: mpsc::Receiver<Notification>,
    interaction: Cell<Interaction>,
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
        }
    }
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
}

#[derive(Debug, Clone, Copy)]
enum Interaction {
    None,
    Connecting(ConnectorKind),
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
    state: T,
    position: Point,
    inputs: Vec<InputId>,
    outputs: Vec<OutputId>,
    links: canvas::Cache<Renderer>,
}

impl<T, Renderer> Clone for State<T, Renderer>
where
    T: Clone,
    Renderer: geometry::Renderer,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            position: self.position,
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            links: canvas::Cache::new(),
        }
    }
}

impl<T> Graph<T> {
    pub fn new(evaluate: fn(&mut T, Data<'_>)) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            nodes: HashMap::new(),
            links: HashMap::new(),
            values: HashMap::new(),
            current: 0,
            evaluate,
            sender,
            receiver,
            interaction: Cell::new(Interaction::None),
        }
    }

    pub fn get(&self, node: Node) -> Option<&T> {
        self.nodes.get(&node).map(|node| &node.state)
    }

    pub fn update<O>(&mut self, node: Node, f: impl FnOnce(&mut T) -> O) -> O
    where
        O: Default,
    {
        let result = {
            let Some(node) = self.nodes.get_mut(&node) else {
                return O::default();
            };

            f(&mut node.state)
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

    pub fn build(&mut self) -> Builder<'_, T> {
        let node = Node(self.current);
        self.current += 1;

        Builder {
            graph: self,
            node,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn push(&mut self, position: impl Into<Point>, f: impl FnOnce(&mut Builder<'_, T>) -> T) {
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
            node.position = position.into();
        }
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

pub struct Builder<'a, T> {
    graph: &'a mut Graph<T>,
    node: Node,
    inputs: Vec<InputId>,
    outputs: Vec<OutputId>,
}

impl<T> Builder<'_, T> {
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

    pub fn finish(self, position: impl Into<Point>, state: T) {
        let _ = self.graph.nodes.insert(
            self.node,
            State {
                state,
                position: position.into(),
                inputs: self.inputs,
                outputs: self.outputs,
                links: canvas::Cache::new(),
            },
        );

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
