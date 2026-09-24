// macOS driver used to produce evidence for docs/gates/macos.md.
//
// Everything here synthesises input at the *window-server* level (CGEvent) or
// reads state through public AppKit/Carbon APIs, so what it observes is what a
// human would see — not a reading of the source under test.
//
// Subcommands:
//   windows                     print on-screen windows (name, pid, bounds)
//   activate <pid>              bring that process frontmost
//   click <x> <y>               real mouse click at screen coordinates
//   move <x> <y>                mouse move only
//   type <text>                 type text (CGEvent unicode), one char per event
//   vkey <code> [flags]         raw virtual keycode down/up (IME sees these)
//   pb-read                     read NSPasteboard text (cross-process truth)
//   pb-write <text>             write NSPasteboard text
//   ax <pid> [depth]            dump the accessibility tree of that process
//   im-list                     list enabled input sources
//   im-select <id-contains>     select an input source by substring of its id

import AppKit
import Carbon
import CoreGraphics

let args = Array(CommandLine.arguments.dropFirst())
guard let cmd = args.first else {
    FileHandle.standardError.write("usage: drv <cmd> ...\n".data(using: .utf8)!)
    exit(2)
}

func out(_ s: String) { print(s) }

// MARK: - windows / activation

func windows() {
    guard let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] else {
        out("no window list")
        return
    }
    for w in list {
        let owner = w[kCGWindowOwnerName as String] as? String ?? "?"
        let pid = w[kCGWindowOwnerPID as String] as? Int32 ?? 0
        let name = w[kCGWindowName as String] as? String ?? ""
        var rect = "(none)"
        if let b = w[kCGWindowBounds as String] as? [String: Any],
           let x = b["X"] as? Double, let y = b["Y"] as? Double,
           let ww = b["Width"] as? Double, let hh = b["Height"] as? Double {
            rect = "x:\(Int(x)) y:\(Int(y)) w:\(Int(ww)) h:\(Int(hh))"
        }
        let layer = w[kCGWindowLayer as String] as? Int ?? 0
        let wid = w[kCGWindowNumber as String] as? Int ?? 0
        out("pid=\(pid) wid=\(wid) owner=\(owner) title=\(name) \(rect) layer=\(layer)")
    }
}

func activate(pid: Int32) {
    guard let app = NSRunningApplication(processIdentifier: pid) else {
        out("activate: no app with pid \(pid)")
        return
    }
    app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
    usleep(400_000)
    out("activated pid \(pid) front=\(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0)")
}

// MARK: - pointer / keys

func post(event: CGEvent) {
    event.post(tap: .cghidEventTap)
    usleep(40_000)
}

func click(x: Double, y: Double) {
    let pt = CGPoint(x: x, y: y)
    let move = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
                       mouseCursorPosition: pt, mouseButton: .left)
    let down = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown,
                       mouseCursorPosition: pt, mouseButton: .left)
    let up = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp,
                     mouseCursorPosition: pt, mouseButton: .left)
    post(event: move!)
    usleep(50_000)
    post(event: down!)
    post(event: up!)
    out("clicked \(Int(x)),\(Int(y))")
}

func movePointer(x: Double, y: Double) {
    let pt = CGPoint(x: x, y: y)
    post(event: CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
                        mouseCursorPosition: pt, mouseButton: .left)!)
    out("moved \(Int(x)),\(Int(y))")
}

func typeUnicode(_ text: String) {
    for ch in text {
        let scalars = String(ch).utf16.map { UniChar($0) }
        guard let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
              let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else { continue }
        down.keyboardSetUnicodeString(stringLength: scalars.count, unicodeString: scalars)
        up.keyboardSetUnicodeString(stringLength: scalars.count, unicodeString: scalars)
        post(event: down)
        post(event: up)
    }
    out("typed \(text.count) char(s)")
}

// Flags: mask understood by postMask below (shift=1 ctrl=2 alt=4 cmd=8).
func vkey(code: UInt16, mask: UInt64) {
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: false) else {
        out("vkey: cannot create event")
        return
    }
    if mask & 1 != 0 { down.flags.insert(.maskShift); up.flags.insert(.maskShift) }
    if mask & 2 != 0 { down.flags.insert(.maskControl); up.flags.insert(.maskControl) }
    if mask & 4 != 0 { down.flags.insert(.maskAlternate); up.flags.insert(.maskAlternate) }
    if mask & 8 != 0 { down.flags.insert(.maskCommand); up.flags.insert(.maskCommand) }
    post(event: down)
    post(event: up)
    out("vkey \(code) mask=\(mask)")
}

// MARK: - pasteboard (cross-process truth)

func pbRead() {
    let pb = NSPasteboard.general
    let text = pb.string(forType: .string) ?? ""
    out("pasteboard=\(text)")
}

func pbWrite(_ text: String) {
    let pb = NSPasteboard.general
    pb.clearContents()
    let ok = pb.setString(text, forType: .string)
    out("pasteboard write ok=\(ok)")
}

// MARK: - accessibility tree

func attrString(_ el: AXUIElement, _ attr: String) -> String {
    var value: CFTypeRef?
    let err = AXUIElementCopyAttributeValue(el, attr as CFString, &value)
    guard err == .success, let v = value else { return "" }
    if let s = v as? String { return s }
    return "\(v)"
}

func axDump(_ el: AXUIElement, _ depth: Int, _ maxDepth: Int, _ lines: inout [String]) {
    if depth > maxDepth { return }
    let role = attrString(el, kAXRoleAttribute)
    let title = attrString(el, kAXTitleAttribute)
    let desc = attrString(el, kAXDescriptionAttribute)
    let value = attrString(el, kAXValueAttribute)
    let subrole = attrString(el, kAXSubroleAttribute)
    let pad = String(repeating: "  ", count: depth)
    lines.append("\(pad)role=\(role) subrole=\(subrole) title=\(title) desc=\(desc) value=\(value)")
    var children: CFTypeRef?
    let err = AXUIElementCopyAttributeValue(el, kAXChildrenAttribute as CFString, &children)
    if err == .success, let kids = children as? [AXUIElement] {
        for kid in kids { axDump(kid, depth + 1, maxDepth, &lines) }
    }
}

func axTree(pid: Int32, maxDepth: Int) {
    let appEl = AXUIElementCreateApplication(pid)
    var lines: [String] = []
    // Ask for a trusted-range read first: untrusted processes see no children.
    let role = attrString(appEl, kAXRoleAttribute)
    out("ax root role=\(role)")
    axDump(appEl, 0, maxDepth, &lines)
    for l in lines { out(l) }
    out("ax nodes=\(lines.count)")
}

// MARK: - input sources (IME)

func imList() {
    guard let sources = TISCreateInputSourceList(nil, true)?.takeRetainedValue() as? [TISInputSource] else {
        out("im-list: none")
        return
    }
    for src in sources {
        let id = src.id
        let name = src.name
        let lang = src.language ?? ""
        if src.selectable { out("im id=\(id) name=\(name) lang=\(lang)") }
    }
}

func imSelect(_ needle: String) {
    guard let sources = TISCreateInputSourceList(nil, true)?.takeRetainedValue() as? [TISInputSource] else { return }
    for src in sources where src.id.contains(needle) {
        TISSelectInputSource(src)
        usleep(200_000)
        out("im selected \(src.id)")
        return
    }
    out("im-select: no match for \(needle)")
}

func imCurrent() {
    let src = TISCopyCurrentKeyboardInputSource().takeRetainedValue()
    out("current im id=\(src.id) name=\(src.name)")
}

// MARK: - dispatch

let a = args
switch cmd {
case "windows": windows()
case "activate": activate(pid: Int32(a[1]) ?? 0)
case "click": click(x: Double(a[1]) ?? 0, y: Double(a[2]) ?? 0)
case "move": movePointer(x: Double(a[1]) ?? 0, y: Double(a[2]) ?? 0)
case "type": typeUnicode(a[1])
case "vkey": vkey(code: UInt16(a[1]) ?? 0, mask: UInt64(a.count > 2 ? Int(a[2]) ?? 0 : 0))
case "pb-read": pbRead()
case "pb-write": pbWrite(a[1])
case "ax": axTree(pid: Int32(a[1]) ?? 0,
                   maxDepth: Int(a.count > 2 ? Int(a[2]) ?? 6 : 6))
case "im-list": imList()
case "trust":
    let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
    out("ax trusted=\(AXIsProcessTrustedWithOptions(opts))")
    out("screen-recording hint: CGRequestScreenCaptureAccess=\(CGRequestScreenCaptureAccess())")
case "im-select": imSelect(a[1])
case "im-current": imCurrent()
default:
    FileHandle.standardError.write("unknown cmd \(cmd)\n".data(using: .utf8)!)
    exit(2)
}

extension TISInputSource {
    var id: String {
        let v = TISGetInputSourceProperty(self, kTISPropertyInputSourceID)
        return (v == nil) ? "" : (Unmanaged<CFString>.fromOpaque(v!).takeUnretainedValue() as String)
    }
    var name: String {
        let v = TISGetInputSourceProperty(self, kTISPropertyLocalizedName)
        return (v == nil) ? "" : (Unmanaged<CFString>.fromOpaque(v!).takeUnretainedValue() as String)
    }
    var language: String? {
        guard let langs = TISGetInputSourceProperty(self, kTISPropertyInputSourceLanguages) else { return nil }
        let arr = Unmanaged<CFArray>.fromOpaque(langs).takeUnretainedValue() as? [String]
        return arr?.first
    }
    var selectable: Bool {
        guard let cats = TISGetInputSourceProperty(self, kTISPropertyInputSourceCategory) else { return false }
        let cat = Unmanaged<CFString>.fromOpaque(cats).takeUnretainedValue() as String
        return cat == (kTISCategoryKeyboardInputSource as String)
    }
}
