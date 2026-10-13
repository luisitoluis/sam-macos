import AppKit

let arguments = CommandLine.arguments
guard arguments.count == 3 else {
    fputs("usage: set-icon.swift <binary> <png>\n", stderr)
    exit(1)
}

let binary = arguments[1]
let png = arguments[2]
guard let image = NSImage(contentsOfFile: png), image.isValid else {
    fputs("could not read icon \(png)\n", stderr)
    exit(1)
}

if !NSWorkspace.shared.setIcon(image, forFile: binary, options: []) {
    fputs("could not set icon on \(binary)\n", stderr)
    exit(1)
}
