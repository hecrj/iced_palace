use crate::core::Widget;

#[derive(Debug, Clone)]
pub enum Event<T, M> {
    Set(T),
    Message(M),
}

pub fn capsule<'a, T, W, Message, Theme, Renderer>(
    f: impl Fn(T) -> W + 'a,
) -> impl Widget<Message, Theme, Renderer> + 'a
where
    T: Copy + Default + 'static,
    W: Widget<Event<T, Message>, Theme, Renderer> + 'a,
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    struct Capsule<F, T> {
        f: F,
        state: std::marker::PhantomData<T>,
    }

    impl<'a, T, F, W, Message, Theme, Renderer> iced_widget::Component<'a, Message, Theme, Renderer>
        for Capsule<F, T>
    where
        T: Copy + Default + 'static,
        F: Fn(T) -> W,
        W: Widget<Event<T, Message>, Theme, Renderer> + 'a,
        Message: 'static,
    {
        type State = T;
        type Event = Event<T, Message>;

        fn update(
            &self,
            state: &mut Self::State,
            event: Self::Event,
            _renderer: &Renderer,
        ) -> Option<Message> {
            match event {
                Event::Set(new) => {
                    *state = new;
                    None
                }
                Event::Message(message) => Some(message),
            }
        }

        fn view(&self, state: &Self::State) -> impl Widget<Self::Event, Theme, Renderer> + 'a {
            (self.f)(*state)
        }
    }

    iced_widget::component(Capsule {
        f,
        state: std::marker::PhantomData,
    })
}
