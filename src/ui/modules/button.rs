use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, rgb,
};

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    child: Option<AnyElement>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id: ElementId = id.into();
        Self {
            id,
            on_click: None,
            child: None,
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let on_click = self.on_click.clone();
        let child = self.child;

        div()
            .id(self.id.clone())
            // .border_1()
            // .border_color(gpui::black())
            .rounded_3xl()
            .bg(rgb(0x88b7e7))
            .w_16()
            .h_16()
            .flex()
            .justify_center()
            .items_center()
            .text_color(gpui::white())
            .hover(|style| style.bg(rgb(0x98acc1)))
            .when_some(child, |this, element| this.child(element))
            // .child(
            //     svg()
            //         .path(icons::STOP_FILLED)
            //         .w(px(26.0))
            //         .h(px(26.0))
            //         .text_color(gpui::white()),
            // )
            .when_some(on_click, |this, on_click| {
                this.on_click(
                    move |event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        on_click(event, window, cx);
                    },
                )
            })
    }
}
