//! In-app previews for image and PDF attachments.
//!
//! Clicking the name of a previewable attachment opens it over the list
//! instead of handing it to another app. Images load through GPUI's own image
//! element; a PDF is rasterised to a PNG with PDFKit and shown the same way,
//! so both live in one card with the same close and open controls.

use std::sync::Arc;

use gpui::{
    AnyElement, Context, Image, IntoElement, ObjectFit, ParentElement, Styled, Window, div, img,
    prelude::*, px, rgba,
};

use super::icons::Icon;
use super::widgets::TipExt;
use super::{FlowlistApp, icons, widgets};

/// The first page of a PDF, rasterised for the preview card, plus the page
/// count so the footer can say how much is behind it.
pub(super) enum PreviewKind {
    Image,
    Pdf {
        page: Arc<Image>,
        pages: usize,
        /// Zero-based page on screen.
        at: usize,
    },
}

pub(super) struct Preview {
    pub path: String,
    pub name: String,
    pub kind: PreviewKind,
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff"];

fn extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Whether double-clicking an attachment should open the in-app card instead
/// of the default handler. Links and unknown file types keep the old path.
pub(super) fn previewable(path: &str) -> bool {
    if flowlist_core::model::attachment::is_web_link(path) {
        return false;
    }
    let extension = extension(path);
    extension == "pdf" || IMAGE_EXTENSIONS.contains(&extension.as_str())
}

impl FlowlistApp {
    /// Open a local image or PDF over the list. Links and unsupported files
    /// keep opening in the app that owns them, with the usual toast.
    pub(super) fn preview_attachment(&mut self, path: String, cx: &mut Context<Self>) {
        let extension = extension(&path);
        let kind = if extension == "pdf" {
            match pdf_page(&path, 0) {
                Some((page, pages)) => PreviewKind::Pdf { page, pages, at: 0 },
                None => {
                    self.toast = Some(format!("Could not preview {}", attachment_name(&path)));
                    self.open_attachment(path);
                    cx.notify();
                    return;
                }
            }
        } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            PreviewKind::Image
        } else {
            self.open_attachment(path);
            cx.notify();
            return;
        };
        self.preview = Some(Preview {
            name: attachment_name(&path),
            path,
            kind,
        });
        cx.notify();
    }

    pub(super) fn close_preview(&mut self, cx: &mut Context<Self>) {
        if self.preview.take().is_some() {
            cx.notify();
        }
    }

    /// `o` in the preview hands the file to its default app, which is the old
    /// click behaviour kept one keystroke away.
    /// Step a multi-page PDF without leaving the card.
    pub(super) fn step_preview_page(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(Preview {
            path,
            kind: PreviewKind::Pdf { page, pages, at },
            ..
        }) = &mut self.preview
        else {
            return;
        };
        let last = pages.saturating_sub(1) as i64;
        let next = (i64::try_from(*at).unwrap_or(0) + delta as i64).clamp(0, last);
        let next = usize::try_from(next).unwrap_or(0);
        if next != *at
            && let Some((rendered, _)) = pdf_page(path, next)
        {
            *page = rendered;
            *at = next;
        }
        cx.notify();
    }

    pub(super) fn open_preview_external(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = &self.preview {
            let path = preview.path.clone();
            self.open_attachment(path);
            cx.notify();
        }
    }

    pub(super) fn render_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme;
        let Some(preview) = &self.preview else {
            return div().into_any_element();
        };
        let (window_width, window_height) = self.window_size.unwrap_or((1254.0, 1404.0));
        let card_width = (window_width - 120.0).clamp(320.0, 980.0);
        let card_height = (window_height - 140.0).clamp(240.0, 780.0);
        let is_pdf = matches!(preview.kind, PreviewKind::Pdf { .. });
        let path = preview.path.clone();
        let name = preview.name.clone();
        let caption = match &preview.kind {
            PreviewKind::Image => {
                let extension = extension(&preview.path).to_ascii_uppercase();
                if extension.is_empty() {
                    "Image".to_string()
                } else {
                    extension
                }
            }
            PreviewKind::Pdf { pages, at, .. } => {
                let page = (*at + 1).min(*pages);
                format!("Page {page} of {pages}")
            }
        };
        let body = match &preview.kind {
            // A plain String is an embedded asset name in gpui; paths must go
            // through PathBuf so the loader reads the file from disk.
            PreviewKind::Image => image(std::path::PathBuf::from(&path)),
            PreviewKind::Pdf { page, .. } => image(page.clone()),
        };
        div()
            .id("preview-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // Dim and swallow clicks meant for the list underneath.
            .occlude()
            .bg(rgba(0x00000066))
            .on_click(cx.listener(|this, _, _, cx| this.close_preview(cx)))
            .child(
                div()
                    .id("preview-card")
                    .flex()
                    .flex_col()
                    .w(px(card_width))
                    .h(px(card_height))
                    .rounded_xl()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.raised)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border_soft)
                            .child(icons::icon(
                                if is_pdf { Icon::Book } else { Icon::Paperclip },
                                14.0,
                                theme.text_faint,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.text)
                                    .child(widgets::fit_head(
                                        &name,
                                        card_width - 120.0,
                                        14.0,
                                        gpui::FontWeight::SEMIBOLD,
                                        theme.text,
                                        window,
                                    )),
                            )
                            .child(
                                div()
                                    .id("preview-close")
                                    .flex_shrink_0()
                                    .size(px(24.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .text_color(theme.text_faint)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(theme.hover).text_color(theme.text))
                                    .flow_tip(self, cx, theme, "Close preview (Esc)")
                                    .child(icons::icon(Icon::Close, 13.0, theme.text_faint))
                                    .on_click(cx.listener(|this, _, _, cx| this.close_preview(cx))),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p_4()
                            .bg(theme.field)
                            .child(body),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_4()
                            .py_2p5()
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .when(
                                        matches!(&preview.kind, PreviewKind::Pdf { pages, .. } if *pages > 1),
                                        |this| {
                                            this.child(
                                                div()
                                                    .id("preview-prev")
                                                    .flex_shrink_0()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(theme.text_dim)
                                                    .hover(|style| style.text_color(theme.text))
                                                    .flow_tip(self, cx, theme, "Previous page (←)")
                                                    .child("‹")
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.step_preview_page(-1, cx)
                                                    })),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.text_faint)
                                                    .child(caption.clone()),
                                            )
                                            .child(
                                                div()
                                                    .id("preview-next")
                                                    .flex_shrink_0()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .text_color(theme.text_dim)
                                                    .hover(|style| style.text_color(theme.text))
                                                    .flow_tip(self, cx, theme, "Next page (→)")
                                                    .child("›")
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.step_preview_page(1, cx)
                                                    })),
                                            )
                                        },
                                    )
                                    .when(
                                        !matches!(&preview.kind, PreviewKind::Pdf { pages, .. } if *pages > 1),
                                        |this| {
                                            this.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.text_faint)
                                                    .child(caption.clone()),
                                            )
                                        },
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_xs()
                                    .text_color(theme.text_faint)
                                    .child(path),
                            )
                            .child(
                                div()
                                    .id("preview-open")
                                    .flex_shrink_0()
                                    .child(widgets::button(theme, "Open externally", false))
                                    .flow_tip(self, cx, theme, "Open in its default app (o)")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.open_preview_external(cx)
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .id("preview-done")
                                    .flex_shrink_0()
                                    .child(widgets::button(theme, "Done", true))
                                    .on_click(cx.listener(|this, _, _, cx| this.close_preview(cx))),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn image(source: impl Into<gpui::ImageSource>) -> AnyElement {
    img(source)
        .max_w_full()
        .max_h_full()
        .object_fit(ObjectFit::Contain)
        .into_any_element()
}

fn attachment_name(path: &str) -> String {
    flowlist_core::model::attachment::attachment_name(path)
}

#[cfg(target_os = "macos")]
fn pdf_page(path: &str, index: usize) -> Option<(Arc<Image>, usize)> {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    #[repr(C)]
    struct NSSize {
        width: f64,
        height: f64,
    }

    #[link(name = "PDFKit", kind = "framework")]
    unsafe extern "C" {}

    let cstring = std::ffi::CString::new(path).ok()?;
    unsafe {
        let ns_path: *mut Object =
            msg_send![class!(NSString), stringWithUTF8String: cstring.as_ptr()];
        let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath: ns_path];
        let document: *mut Object = msg_send![class!(PDFDocument), alloc];
        let document: *mut Object = msg_send![document, initWithURL: url];
        if document.is_null() {
            return None;
        }
        let pages: usize = msg_send![document, pageCount];
        if pages == 0 {
            return None;
        }
        let page: *mut Object = msg_send![document, pageAtIndex: index];
        if page.is_null() {
            return None;
        }
        // kPDFDisplayBoxMediaBox. A generous size keeps a full page legible
        // when the card is wide; the element scales it down to fit.
        let size = NSSize {
            width: 1600.0,
            height: 2000.0,
        };
        let thumbnail: *mut Object = msg_send![page, thumbnailOfSize: size forBox: 0usize];
        if thumbnail.is_null() {
            return None;
        }
        let tiff: *mut Object = msg_send![thumbnail, TIFFRepresentation];
        if tiff.is_null() {
            return None;
        }
        let bitmap: *mut Object = msg_send![class!(NSBitmapImageRep), imageRepWithData: tiff];
        if bitmap.is_null() {
            return None;
        }
        let properties: *mut Object = msg_send![class!(NSDictionary), dictionary];
        // NSBitmapImageFileTypePNG.
        let png: *mut Object = msg_send![
            bitmap,
            representationUsingType: 4usize
            properties: properties
        ];
        if png.is_null() {
            return None;
        }
        let length: usize = msg_send![png, length];
        let bytes: *const u8 = msg_send![png, bytes];
        if bytes.is_null() || length == 0 {
            return None;
        }
        let bytes = std::slice::from_raw_parts(bytes, length).to_vec();
        // The document is ours from alloc/init; let the pool reclaim it.
        let _: () = msg_send![document, autorelease];
        Some((
            Arc::new(Image::from_bytes(gpui::ImageFormat::Png, bytes)),
            pages,
        ))
    }
}

#[cfg(not(target_os = "macos"))]
fn pdf_page(_path: &str, _index: usize) -> Option<(Arc<Image>, usize)> {
    None
}
