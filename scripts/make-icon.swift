// Draw the Flowlist app icon at 1024×1024.
//
// Usage: swift scripts/make-icon.swift <output.png>
//
// The glyph is deliberately simple: a blue field, three checklist rows and
// check marks. Everything is drawn with Core Graphics so the script has no
// dependency beyond the macOS SDK.

import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let output = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "icon.png"
let side = 1024
let space = CGColorSpaceCreateDeviceRGB()
guard
    let context = CGContext(
        data: nil,
        width: side,
        height: side,
        bitsPerComponent: 8,
        bytesPerRow: 0,
        space: space,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    )
else {
    fatalError("could not create the icon context")
}

context.setAllowsAntialiasing(true)
context.setShouldAntialias(true)

// Rounded field with a soft vertical gradient.
let field = CGRect(x: 72, y: 72, width: 880, height: 880)
let rounded = CGPath(
    roundedRect: field,
    cornerWidth: 200,
    cornerHeight: 200,
    transform: nil
)
context.saveGState()
context.addPath(rounded)
context.clip()
let gradient = CGGradient(
    colorsSpace: space,
    colors: [
        CGColor(red: 0.24, green: 0.51, blue: 0.80, alpha: 1),
        CGColor(red: 0.13, green: 0.35, blue: 0.62, alpha: 1),
    ] as CFArray,
    locations: [0, 1]
)!
context.drawLinearGradient(
    gradient,
    start: CGPoint(x: 512, y: 952),
    end: CGPoint(x: 512, y: 72),
    options: []
)
context.restoreGState()

// Three checklist rows.
for index in 0..<3 {
    let y = 700 - CGFloat(index) * 170
    let center = CGPoint(x: 300, y: y)

    // Ring.
    context.setStrokeColor(CGColor(red: 1, green: 1, blue: 1, alpha: 0.9))
    context.setLineWidth(26)
    context.addArc(
        center: center,
        radius: 62,
        startAngle: 0,
        endAngle: .pi * 2,
        clockwise: false
    )
    context.strokePath()

    // Check mark.
    context.setLineWidth(28)
    context.setLineCap(.round)
    context.setLineJoin(.round)
    context.move(to: CGPoint(x: center.x - 32, y: center.y - 2))
    context.addLine(to: CGPoint(x: center.x - 8, y: center.y - 28))
    context.addLine(to: CGPoint(x: center.x + 38, y: center.y + 22))
    context.strokePath()

    // Label bar; rows shorten slightly so the icon reads as a list.
    let width: CGFloat = index == 2 ? 300 : 340
    let bar = CGRect(x: 420, y: y - 27, width: width, height: 54)
    let barPath = CGPath(
        roundedRect: bar,
        cornerWidth: 27,
        cornerHeight: 27,
        transform: nil
    )
    context.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 0.92))
    context.addPath(barPath)
    context.fillPath()
}

guard let image = context.makeImage() else {
    fatalError("could not render the icon")
}
let url = URL(fileURLWithPath: output)
guard
    let destination = CGImageDestinationCreateWithURL(
        url as CFURL,
        UTType.png.identifier as CFString,
        1,
        nil
    )
else {
    fatalError("could not create \(output)")
}
CGImageDestinationAddImage(destination, image, nil)
guard CGImageDestinationFinalize(destination) else {
    fatalError("could not write \(output)")
}
