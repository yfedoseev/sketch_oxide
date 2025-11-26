# Installation Guide

Choose your language and follow the installation instructions.

## Rust

### Via Cargo

Add to your `Cargo.toml`:

```toml
[dependencies]
sketch_oxide = "0.1"
```

Then in your code:

```rust
use sketch_oxide::cardinality::HyperLogLog;

fn main() {
    let mut hll = HyperLogLog::new(14).unwrap();
    hll.update(&"hello".as_bytes());
    println!("Estimated cardinality: {}", hll.estimate());
}
```

### Building from Source

```bash
git clone https://github.com/sketchoxide/sketch_oxide
cd sketch_oxide/sketch_oxide
cargo build --release
```

## Python

### Via pip

```bash
pip install sketch-oxide
```

### From Source

```bash
git clone https://github.com/sketchoxide/sketch_oxide
cd sketch_oxide
pip install -e ./python
```

### Verify Installation

```python
from sketch_oxide import HyperLogLog

hll = HyperLogLog(precision=14)
hll.update(b"hello")
print(f"Estimated cardinality: {hll.estimate()}")
```

## Java

### Via Maven

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>io.sketchoxide</groupId>
    <artifactId>sketch-oxide</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Via Gradle

```gradle
dependencies {
    implementation 'io.sketchoxide:sketch-oxide:0.1.0'
}
```

### From Source

```bash
git clone https://github.com/sketchoxide/sketch_oxide
cd sketch_oxide/java
mvn clean install
```

### Verify Installation

```java
import io.sketchoxide.cardinality.HyperLogLog;

public class Example {
    public static void main(String[] args) {
        try (HyperLogLog hll = new HyperLogLog(14)) {
            hll.update("hello".getBytes());
            System.out.println("Estimated cardinality: " + hll.estimate());
        }
    }
}
```

## Node.js / TypeScript

### Via npm

```bash
npm install @sketchoxide/core
```

### Via yarn

```bash
yarn add @sketchoxide/core
```

### From Source

```bash
git clone https://github.com/sketchoxide/sketch_oxide
cd sketch_oxide/nodejs
npm install
npm run build
```

### Verify Installation

```typescript
import { HyperLogLog } from '@sketchoxide/core';

const hll = new HyperLogLog(14);
hll.update(Buffer.from('hello'));
console.log(`Estimated cardinality: ${hll.estimate()}`);
```

## C# / .NET

### Via NuGet

```bash
dotnet add package SketchOxide
```

### Via Package Manager

```
Install-Package SketchOxide
```

### From Source

```bash
git clone https://github.com/sketchoxide/sketch_oxide
cd sketch_oxide/dotnet
dotnet build -c Release
```

### Verify Installation

```csharp
using SketchOxide.Cardinality;

class Program {
    static void Main() {
        using (var hll = new HyperLogLog(14)) {
            hll.Update("hello"u8);
            Console.WriteLine($"Estimated cardinality: {hll.Estimate()}");
        }
    }
}
```

## Troubleshooting

### Rust

- **Cargo not found**: Install Rust from [rustup.rs](https://rustup.rs)
- **Compilation errors**: Ensure you have the latest Rust version: `rustup update`

### Python

- **ModuleNotFoundError**: Ensure pip installed the package in the correct Python environment
- **Native library issues**: Some systems may require `gcc` or `clang`: `apt-get install build-essential`

### Java

- **Maven not found**: Install Maven from [maven.apache.org](https://maven.apache.org)
- **JNI issues**: Ensure JAVA_HOME environment variable is set

### Node.js

- **Build errors**: Ensure Node.js 14+ is installed and npm is up to date
- **Native module issues**: May need `python3` and build tools: `npm install --global build-essential`

### C# / .NET

- **.NET SDK not found**: Install from [dotnet.microsoft.com](https://dotnet.microsoft.com)
- **NuGet restore issues**: Clear cache: `dotnet nuget locals all --clear`

## Platform Support

| Language | Platforms | Status |
|----------|-----------|--------|
| Rust | Linux, macOS, Windows | ✅ Full |
| Python | Linux, macOS, Windows (3.8+) | ✅ Full |
| Java | Linux, macOS, Windows (JDK 11+) | ✅ Full |
| Node.js | Linux, macOS, Windows (Node 14+) | ✅ Full |
| C# / .NET | Linux, macOS, Windows (.NET 6.0+) | ✅ Full |

## Next Steps

- Read the [Quick Start Guide](quick-start.md)
- Choose an algorithm in [Choosing an Algorithm](choosing-algorithm.md)
- Check language-specific guides in [Languages](../languages/)
