//! Display a dropdown list of selectable values.
use crate::{
    layout, mouse, overlay,
    overlay::menu::{self, Menu},
    scrollable, text, Clipboard, Element, Event, Hasher, Layout, Length, Point,
    Rectangle, Size, Widget,
};
use std::borrow::Cow;

/// A widget for selecting a single value from a list of options.
#[allow(missing_debug_implementations)]
pub struct PickList<'a, T, Message, Renderer: self::Renderer>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    menu: &'a mut menu::State,
    is_open: &'a mut bool,
    hovered_option: &'a mut Option<usize>,
    last_selection: &'a mut Option<T>,
    on_selected: Box<dyn Fn(T) -> Message>,
    options: Cow<'a, [T]>,
    selected: Option<T>,
    width: Length,
    padding: u16,
    text_size: Option<u16>,
    font: Renderer::Font,
    style: <Renderer as self::Renderer>::Style,
}

/// The local state of a [`PickList`].
///
/// [`PickList`]: struct.PickList.html
#[derive(Debug, Clone)]
pub struct State<T> {
    menu: menu::State,
    is_open: bool,
    hovered_option: Option<usize>,
    last_selection: Option<T>,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            menu: menu::State::default(),
            is_open: bool::default(),
            hovered_option: Option::default(),
            last_selection: Option::default(),
        }
    }
}

impl<'a, T: 'a, Message, Renderer: self::Renderer>
    PickList<'a, T, Message, Renderer>
where
    T: ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
{
    /// Creates a new [`PickList`] with the given [`State`], a list of options,
    /// the current selected value, and the message to produce when an option is
    /// selected.
    ///
    /// [`PickList`]: struct.PickList.html
    /// [`State`]: struct.State.html
    pub fn new(
        state: &'a mut State<T>,
        options: impl Into<Cow<'a, [T]>>,
        selected: Option<T>,
        on_selected: impl Fn(T) -> Message + 'static,
    ) -> Self {
        let State {
            menu,
            is_open,
            hovered_option,
            last_selection,
        } = state;

        Self {
            menu,
            is_open,
            hovered_option,
            last_selection,
            on_selected: Box::new(on_selected),
            options: options.into(),
            selected,
            width: Length::Shrink,
            text_size: None,
            padding: Renderer::DEFAULT_PADDING,
            font: Default::default(),
            style: Default::default(),
        }
    }

    /// Sets the width of the [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets the padding of the [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Sets the text size of the [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    pub fn text_size(mut self, size: u16) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Sets the font of the [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the style of the [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    pub fn style(
        mut self,
        style: impl Into<<Renderer as self::Renderer>::Style>,
    ) -> Self {
        self.style = style.into();
        self
    }
}

impl<'a, T: 'a, Message, Renderer> Widget<Message, Renderer>
    for PickList<'a, T, Message, Renderer>
where
    T: Clone + ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
    Message: 'static,
    Renderer: self::Renderer + scrollable::Renderer + 'a,
{
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        use std::f32;

        let limits = limits
            .width(self.width)
            .height(Length::Shrink)
            .pad(f32::from(self.padding));

        let text_size = self.text_size.unwrap_or(renderer.default_size());

        let max_width = match self.width {
            Length::Shrink => {
                let labels = self.options.iter().map(ToString::to_string);

                labels
                    .map(|label| {
                        let (width, _) = renderer.measure(
                            &label,
                            text_size,
                            Renderer::Font::default(),
                            Size::new(f32::INFINITY, f32::INFINITY),
                        );

                        width.round() as u32
                    })
                    .max()
                    .unwrap_or(100)
            }
            _ => 0,
        };

        let size = {
            let intrinsic = Size::new(
                max_width as f32
                    + f32::from(text_size)
                    + f32::from(self.padding),
                f32::from(text_size),
            );

            limits.resolve(intrinsic).pad(f32::from(self.padding))
        };

        layout::Node::new(size)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        use std::hash::Hash as _;

        match self.width {
            Length::Shrink => {
                self.options
                    .iter()
                    .map(ToString::to_string)
                    .for_each(|label| label.hash(state));
            }
            _ => {
                self.width.hash(state);
            }
        }
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if *self.is_open {
                    // TODO: Encode cursor availability in the type system
                    *self.is_open =
                        cursor_position.x < 0.0 || cursor_position.y < 0.0;
                } else if layout.bounds().contains(cursor_position) {
                    let selected = self.selected.as_ref();

                    *self.is_open = true;
                    *self.hovered_option = self
                        .options
                        .iter()
                        .position(|option| Some(option) == selected);
                }

                if let Some(last_selection) = self.last_selection.take() {
                    messages.push((self.on_selected)(last_selection));

                    *self.is_open = false;
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _defaults: &Renderer::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> Renderer::Output {
        self::Renderer::draw(
            renderer,
            layout.bounds(),
            cursor_position,
            self.selected.as_ref().map(ToString::to_string),
            self.padding,
            self.text_size.unwrap_or(renderer.default_size()),
            self.font,
            &self.style,
        )
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>,
    ) -> Option<overlay::Element<'_, Message, Renderer>> {
        if *self.is_open {
            let bounds = layout.bounds();

            let mut menu = Menu::new(
                &mut self.menu,
                &self.options,
                &mut self.hovered_option,
                &mut self.last_selection,
            )
            .width(bounds.width.round() as u16)
            .padding(self.padding)
            .font(self.font)
            .style(Renderer::menu_style(&self.style));

            if let Some(text_size) = self.text_size {
                menu = menu.text_size(text_size);
            }

            Some(menu.overlay(layout.position(), bounds.height))
        } else {
            None
        }
    }
}

/// The renderer of a [`PickList`].
///
/// Your [renderer] will need to implement this trait before being
/// able to use a [`PickList`] in your user interface.
///
/// [`PickList`]: struct.PickList.html
/// [renderer]: ../../renderer/index.html
pub trait Renderer: text::Renderer + menu::Renderer {
    /// The default padding of a [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    const DEFAULT_PADDING: u16;

    /// The [`PickList`] style supported by this renderer.
    ///
    /// [`PickList`]: struct.PickList.html
    type Style: Default;

    /// Returns the style of the [`Menu`] of the [`PickList`].
    ///
    /// [`Menu`]: ../../overlay/menu/struct.Menu.html
    /// [`PickList`]: struct.PickList.html
    fn menu_style(
        style: &<Self as Renderer>::Style,
    ) -> <Self as menu::Renderer>::Style;

    /// Draws a [`PickList`].
    ///
    /// [`PickList`]: struct.PickList.html
    fn draw(
        &mut self,
        bounds: Rectangle,
        cursor_position: Point,
        selected: Option<String>,
        padding: u16,
        text_size: u16,
        font: Self::Font,
        style: &<Self as Renderer>::Style,
    ) -> Self::Output;
}

impl<'a, T: 'a, Message, Renderer> Into<Element<'a, Message, Renderer>>
    for PickList<'a, T, Message, Renderer>
where
    T: Clone + ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
    Renderer: self::Renderer + 'a,
    Message: 'static,
{
    fn into(self) -> Element<'a, Message, Renderer> {
        Element::new(self)
    }
}
