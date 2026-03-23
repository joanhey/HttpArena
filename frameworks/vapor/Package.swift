// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "httparena-vapor",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(url: "https://github.com/vapor/vapor.git", from: "4.100.0"),
    ],
    targets: [
        .systemLibrary(
            name: "CSQLite",
            path: "Sources/CSQLite",
            pkgConfig: "sqlite3",
            providers: [
                .apt(["libsqlite3-dev"]),
            ]
        ),
        .executableTarget(
            name: "Server",
            dependencies: [
                "CSQLite",
                .product(name: "Vapor", package: "vapor"),
            ],
            path: "src"
        ),
    ]
)
