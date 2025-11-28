use makepad_widgets::*;

live_design!{
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*;

    App = {{App}} {
        ui: <Window>{
            show_bg: true,
            width: Fit,
            height: Fit,
            body = <View> {
                align: {x: 0.5, y: 0.5},
                spacing: 20,
                setup_button = <Button> { text: "Setup" }
                log_button = <Button> { text: "View Logs" }
                quit_button = <Button> { text: "Quit" }
            }
        }
    }
}

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live] ui: WidgetRef,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App{
    fn handle_actions(&mut self, cx: &mut Cx, actions:&Actions){
        if self.ui.button(id!(quit_button)).clicked(actions) {
            cx.quit();
        }
        if self.ui.button(id!(setup_button)).clicked(actions) {
            log!("Setup button clicked!");
        }
        if self.ui.button(id!(log_button)).clicked(actions) {
            log!("Log button clicked!");
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
