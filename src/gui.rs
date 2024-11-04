use cocoa::appkit::{
    NSApp, NSApplication, NSBackingStoreType, NSButton, NSMenu, NSMenuItem, NSWindowButton,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send};
use objc::{sel, sel_impl};
use tao::platform::macos::WindowExtMacOS;

pub fn create_menu_bar(title: &str) {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();

        // Create the main menu first
        let main_menu = NSMenu::new(nil).autorelease();
        let _: () = msg_send![app, setMainMenu:main_menu];

        // Create the application menu
        let app_menu = NSMenu::new(nil).autorelease();
        let app_menu_item = NSMenuItem::new(nil).autorelease();

        // Set the title
        let title_str = NSString::alloc(nil).init_str(title).autorelease();

        // Set process name first
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        let _: () = msg_send![process_info, setProcessName:title_str];

        // Configure menu items
        let _: () = msg_send![app_menu_item, setTitle:title_str];
        let _: () = msg_send![app_menu_item, setSubmenu:app_menu];
        let _: () = msg_send![main_menu, addItem:app_menu_item];

        // Force an update
        let _: () = msg_send![main_menu, update];
        let _: () = msg_send![app, setMainMenu:main_menu];

        set_titles(title);
    }
}

pub fn set_titles(title: &str) {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApp();

        // Create the menu bar if it doesn't exist
        let main_menu = NSMenu::new(nil).autorelease();
        app.setMainMenu_(main_menu);

        // Create the application menu
        let app_menu = NSMenu::new(nil).autorelease();
        let app_menu_item = NSMenuItem::new(nil).autorelease();

        // Create the title as NSString
        let title_str = NSString::alloc(nil).init_str(title).autorelease();

        // Set menu titles
        app_menu.setTitle_(title_str);
        app_menu_item.setTitle_(title_str);
        main_menu.addItem_(app_menu_item);
        app_menu_item.setSubmenu_(app_menu);

        // Set application name
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        let _: () = msg_send![process_info, setProcessName:title_str];

        // Set window title
        if let Some(window) = get_main_window() {
            let _: () = msg_send![window, setTitle:title_str];
        }
    }
}

fn get_main_window() -> Option<id> {
    unsafe {
        let app = NSApp();
        let windows: id = msg_send![app, windows];
        let count: usize = msg_send![windows, count];

        if count > 0 {
            let window: id = msg_send![windows, objectAtIndex:0];
            if window != nil {
                Some(window)
            } else {
                None
            }
        } else {
            None
        }
    }
}

use cocoa::appkit::NSWindowStyleMask;

pub(crate) unsafe fn disable_window_resize(window: &tao::window::Window) {
    let ns_window: id = window.ns_window() as id;

    let current_style_mask: NSWindowStyleMask = msg_send![ns_window, styleMask];
    let new_style_mask = current_style_mask
        & !(NSWindowStyleMask::NSResizableWindowMask
            | NSWindowStyleMask::NSMiniaturizableWindowMask);

    let _: () = msg_send![ns_window, setStyleMask: new_style_mask];
}

pub(crate) unsafe fn show_titlebar_and_controls(window: &tao::window::Window) {
    let ns_window: id = window.ns_window() as id;

    // Set window style mask to include title bar and standard window buttons
    let style_mask = NSWindowStyleMask::NSTitledWindowMask  // Shows title bar
        | NSWindowStyleMask::NSClosableWindowMask; // Shows close button
                                                   // | NSWindowStyleMask::NSMiniaturizableWindowMask     // Shows minimize button
                                                   // | NSWindowStyleMask::NSResizableWindowMask; // Makes window resizable

    let _: () = msg_send![ns_window, setStyleMask: style_mask];

    // Make title bar visible
    let _: () = msg_send![ns_window, setTitlebarAppearsTransparent: NO];
    let _: () = msg_send![ns_window, setTitleVisibility: 0]; // 0 means visible
    let _: () = msg_send![ns_window, setMovableByWindowBackground: YES];

    // Ensure window has shadow
    let _: () = msg_send![ns_window, setHasShadow: YES];

    // Force window to update
    let _: () = msg_send![ns_window, display];
}

pub(crate) unsafe fn make_borderless(window: &tao::window::Window) {
    let ns_window: id = window.ns_window() as id;

    // Create a clear color
    let clear_color: id = msg_send![class!(NSColor), clearColor];

    let style_mask = NSWindowStyleMask::NSBorderlessWindowMask;
    let _: () = msg_send![ns_window, setStyleMask: style_mask];
    let _: () = msg_send![ns_window, setMovableByWindowBackground: YES];
    let _: () = msg_send![ns_window, setTitlebarAppearsTransparent: YES];
    let _: () = msg_send![ns_window, setTitleVisibility: 1];
    let _: () = msg_send![ns_window, setOpaque: YES];
    let _: () = msg_send![ns_window, setHasShadow: YES];

    // Create dark grey color (RGB: 0.2, 0.2, 0.2)
    let dark_grey: id =
        msg_send![class!(NSColor), colorWithCalibratedRed:0.2 green:0.2 blue:0.2 alpha:1.0];
    let _: () = msg_send![ns_window, setBackgroundColor: dark_grey];

    // Configure window transparency
    let _: () = msg_send![ns_window, setAlphaValue: 1.0];
    let _: () = msg_send![ns_window, setBackingType: NSBackingStoreType::NSBackingStoreBuffered];

    // Get the content view and enable layer
    let content_view: id = msg_send![ns_window, contentView];
    let _: () = msg_send![content_view, setWantsLayer: YES];

    // Configure the layer with rounded corners
    let layer: id = msg_send![content_view, layer];
    let _: () = msg_send![layer, setBackgroundColor: clear_color];
    let _: () = msg_send![layer, setMasksToBounds: YES];
    let _: () = msg_send![layer, setCornerRadius: 5.0]; // Adjust this value for different corner radius

    // Get and configure background view
    let background_view: id =
        msg_send![ns_window, standardWindowButton:NSWindowButton::NSWindowCloseButton];
    if background_view != nil {
        let superview: id = msg_send![background_view, superview];
        if superview != nil {
            let background_view_layer: id = msg_send![superview, layer];
            let _: () = msg_send![layer, setBackgroundColor: clear_color];
            let _: () = msg_send![background_view_layer, setCornerRadius: 5.0];
            let _: () = msg_send![background_view_layer, setMasksToBounds: YES];
        }
    }

    // Force window to update
    let _: () = msg_send![ns_window, display];
    let _: () = msg_send![ns_window, invalidateShadow];
    let _: () = msg_send![content_view, setNeedsDisplay: YES];

    // Remove any visual effect views
    if let Some(visual_effect_view) = get_visual_effect_view(ns_window) {
        let _: () = msg_send![visual_effect_view, removeFromSuperview];
    }
}

unsafe fn get_visual_effect_view(window: id) -> Option<id> {
    let content_view: id = msg_send![window, contentView];
    let subviews: id = msg_send![content_view, subviews];
    let count: usize = msg_send![subviews, count];

    for i in 0..count {
        let view: id = msg_send![subviews, objectAtIndex: i];
        let class_name: id = msg_send![view, className];
        let class_name_str = std::ffi::CStr::from_ptr(msg_send![class_name, UTF8String]);
        if let Ok(name) = class_name_str.to_str() {
            if name.contains("NSVisualEffectView") {
                return Some(view);
            }
        }
    }
    None
}
