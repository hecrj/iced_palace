use iced::widget::{column, container, row, text, text_input};
use iced::{Center, Element, Font, Point, Theme};
use iced_palace::widget::node_editor;
use iced_palace::widget::node_editor::{Data, Graph, Input, Link, Node, Output};

use function::Binary;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view)
        .theme(|_| Theme::CatppuccinMacchiato)
        .run()
}

struct Example {
    graph: Graph<Instruction>,
}

#[derive(Debug, Clone)]
enum Message {
    NumberChanged(Node, String),
    NodeDragged(Node, Point),
    GraphLinked(Link),
}

impl Example {
    fn new() -> Self {
        let mut graph = Graph::new(Instruction::evaluate);

        let mut a = graph.build();
        let a_output = a.output("output");
        a.finish(
            (10.0, 10.0),
            Instruction::Number {
                n: 1,
                output: a_output,
            },
        );

        let mut b = graph.build();
        let b_output = b.output("output");
        b.finish(
            (10.0, 200.0),
            Instruction::Number {
                n: 2,
                output: b_output,
            },
        );

        let mut add = graph.build();
        let add_a = add.input("a");
        let add_b = add.input("b");
        let add_result = add.output("result");
        add.finish(
            (300.0, 100.0),
            Instruction::Add {
                a: add_a,
                b: add_b,
                output: add_result,
            },
        );

        let mut display = graph.build();
        let display_value = display.input("value");
        display.finish((300.0, 200.0), Instruction::Display(display_value));

        // graph.link(a_output, add_a);
        // graph.link(b_output, add_b);
        // graph.link(add_result, display_value);

        Self { graph }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::NumberChanged(node, input) => {
                self.graph.update(node, |instruction, _| {
                    let Instruction::Number { n, .. } = instruction else {
                        return;
                    };

                    let Ok(new_number) = input.parse() else {
                        return;
                    };

                    *n = new_number;
                });
            }
            Message::NodeDragged(node, position) => {
                self.graph.move_to(node, position);
            }
            Message::GraphLinked(link) => {
                self.graph.link(link);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        node_editor(&self.graph, |id, node, interface| {
            let title = match node {
                Instruction::Number { .. } => "Number",
                Instruction::Add { .. } => "Add",
                Instruction::Display(_) => "Display",
            };

            let content: Element<'_, _> = match node {
                Instruction::Number { n, .. } => text_input("Type a number", &n.to_string())
                    .on_input(Message::NumberChanged.with(id))
                    .into(),
                Instruction::Add { a, b, .. } => text!(
                    "{} + {}",
                    self.graph
                        .input(a)
                        .map(u32::to_string)
                        .unwrap_or("???".to_owned()),
                    self.graph
                        .input(b)
                        .map(u32::to_string)
                        .unwrap_or("???".to_owned()),
                )
                .into(),
                Instruction::Display(n) => {
                    self.graph.input(n).map(text).unwrap_or(text("???")).into()
                }
            };

            let inputs = interface
                .has_inputs()
                .then(|| column(interface.inputs().map(|input| input.handle())).spacing(10));

            let outputs = interface
                .has_outputs()
                .then(|| column(interface.outputs().map(|output| output.handle())).spacing(10));

            container(
                column![
                    interface.drag_handle(
                        text(title).font(Font::MONOSPACE),
                        Message::NodeDragged.with(id)
                    ),
                    row![inputs, content, outputs].spacing(10).align_y(Center)
                ]
                .align_x(Center)
                .spacing(10),
            )
            .style(container::dark)
            .padding(10)
            .into()
        })
        .on_link(Message::GraphLinked)
        .into()
    }
}

enum Instruction {
    Number {
        n: u32,
        output: Output<u32>,
    },
    Add {
        a: Input<u32>,
        b: Input<u32>,
        output: Output<u32>,
    },
    Display(Input<u32>),
}

impl Instruction {
    fn evaluate(&mut self, mut data: Data<'_>) {
        match self {
            Self::Number { n, output } => data.set(output, *n),
            Self::Add { a, b, output } => {
                data.set_with(output, |data| Some(data.get(a)? + data.get(b)?));
            }
            Self::Display(_) => {}
        }
    }
}
