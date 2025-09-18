# JavaScript Code Samples for Deno Core on Nanvix

Collection of JavaScript demos showcasing deno_core capabilities on nanvix. Each example focuses on specific APIs without fluff.

## Available Demos

### 1. Basic Output (`basic-output.js`)
Console logging and direct printing
```javascript
console.log("Hello nanvix!");
console.error("Error output");
Deno.core.print("Direct stdout\n");
Deno.core.print("Direct stderr\n", true);
```

### 2. Text Encoding (`text-encoding.js`)
UTF-8 encoding/decoding operations
```javascript
const text = "Hello 🦕 nanvix!";
const encoded = Deno.core.encode(text);
const decoded = Deno.core.decode(encoded);

console.log("Original:", text);
console.log("Encoded length:", encoded.length);
console.log("Decoded:", decoded);
console.log("Round-trip success:", text === decoded);
```

### 3. Type Checking (`type-checking.js`)
Runtime type inspection utilities
```javascript
const values = {
  promise: Promise.resolve("test"),
  date: new Date(),
  uint8: new Uint8Array([1,2,3]),
  regex: /test/g,
  map: new Map([["a", 1]]),
  array: [1,2,3]
};

for (const [name, val] of Object.entries(values)) {
  const checks = [];
  if (Deno.core.isPromise(val)) checks.push("Promise");
  if (Deno.core.isDate(val)) checks.push("Date");
  if (Deno.core.isTypedArray(val)) checks.push("TypedArray");
  if (Deno.core.isRegExp(val)) checks.push("RegExp");
  if (Deno.core.isMap(val)) checks.push("Map");
  
  console.log(`${name}: ${checks.join(", ") || "none"}`);
}
```

### 4. Memory Monitoring (`memory-monitor.js`)
Memory usage tracking and statistics
```javascript
function showMemory(label) {
  const mem = Deno.core.memoryUsage();
  console.log(`${label}:`);
  console.log(`  Heap used: ${Math.round(mem.heapUsed/1024)}KB`);
  console.log(`  Heap total: ${Math.round(mem.heapTotal/1024)}KB`);
  console.log(`  External: ${Math.round(mem.external/1024)}KB`);
}

showMemory("Initial");

// Create some objects
const data = Array.from({length: 1000}, (_, i) => ({
  id: i,
  name: `item_${i}`,
  timestamp: Date.now()
}));

showMemory("After creating 1000 objects");

// Test string byte length
const testStr = "Hello 🦕 world!";
console.log(`String "${testStr}" is ${Deno.core.byteLength(testStr)} bytes`);
```

### 5. Serialization (`serialization.js`)
Object serialization and deserialization
```javascript
const complexObj = {
  string: "test data",
  number: 42.5,
  boolean: true,
  array: [1, 2, 3, "hello"],
  nested: {
    deep: {
      value: "buried treasure"
    }
  },
  timestamp: Date.now()
};

console.log("Original object:", JSON.stringify(complexObj));

try {
  const serialized = Deno.core.serialize(complexObj);
  console.log("Serialized size:", serialized.length, "bytes");
  
  const deserialized = Deno.core.deserialize(serialized);
  console.log("Deserialized:", JSON.stringify(deserialized));
  
  console.log("Round-trip success:", JSON.stringify(complexObj) === JSON.stringify(deserialized));
} catch (e) {
  console.error("Serialization error:", e.message);
}
```

### 6. Promise Inspection (`promise-inspection.js`)
Promise state analysis and debugging
```javascript
// Resolved promise
const resolved = Promise.resolve("success value");
const resolvedDetails = Deno.core.getPromiseDetails(resolved);
console.log("Resolved promise:", resolvedDetails);

// Rejected promise
const rejected = Promise.reject(new Error("test error"));
const rejectedDetails = Deno.core.getPromiseDetails(rejected);
console.log("Rejected promise:", rejectedDetails);

// Pending promise
const pending = new Promise(() => {}); // Never resolves
const pendingDetails = Deno.core.getPromiseDetails(pending);
console.log("Pending promise:", pendingDetails);

// Handle rejection to prevent uncaught error
rejected.catch(() => {});
```

### 7. Dynamic Evaluation (`dynamic-eval.js`)
Runtime JavaScript evaluation
```javascript
const expressions = [
  "2 + 3 * 4",
  "Math.PI * 2",
  "[1,2,3].map(x => x * 2)",
  "new Date().getTime()",
  "JSON.stringify({test: true})"
];

expressions.forEach(expr => {
  const [result, error] = Deno.core.evalContext(expr, "<eval>", undefined);
  
  if (error) {
    console.log(`❌ ${expr} -> Error: ${error.thrown.message}`);
  } else {
    console.log(`✅ ${expr} -> ${result}`);
  }
});

// Test error evaluation
const [errorResult, evalError] = Deno.core.evalContext(
  "throw new Error('intentional error')", 
  "<error-test>", 
  undefined
);

console.log("Error test result:", evalError ? `Caught: ${evalError.thrown.message}` : "No error");
```

### 8. Resource Inspection (`resources.js`)
System resource monitoring
```javascript
console.log("System Resources:");
const resources = Deno.core.resources();
console.log("Available resources:", JSON.stringify(resources, null, 2));

console.log("\nRuntime Information:");
console.log("Timer depth:", Deno.core.getTimerDepth());
console.log("Event loop has work:", Deno.core.eventLoopHasMoreWork());

console.log("\nAvailable Operations:");
const ops = Deno.core.opNames();
console.log("Total ops:", ops.length);
console.log("Sample ops:", ops.slice(0, 10).join(", "));
```

### 9. Error Handling (`error-handling.js`)
Built-in error classes demonstration
```javascript
// Test built-in error classes
const errorTests = [
  () => { throw new Deno.core.BadResource("Resource not found"); },
  () => { throw new Deno.core.Interrupted("Operation interrupted"); },
  () => { throw new Deno.core.NotCapable("Permission denied"); }
];

errorTests.forEach((test, i) => {
  try {
    test();
  } catch (e) {
    console.log(`Error ${i+1}: ${e.name} - ${e.message}`);
    console.log(`  Constructor: ${e.constructor.name}`);
    console.log(`  Is BadResource: ${e instanceof Deno.core.BadResource}`);
    console.log(`  Is Interrupted: ${e instanceof Deno.core.Interrupted}`);
    console.log(`  Is NotCapable: ${e instanceof Deno.core.NotCapable}`);
  }
});

// Test regular JavaScript errors
try {
  JSON.parse("invalid json");
} catch (e) {
  console.log(`JS Error: ${e.name} - ${e.message}`);
  
  // Use Deno.core.destructureError for detailed analysis
  const errorDetails = Deno.core.destructureError(e);
  console.log("Error details:", JSON.stringify(errorDetails, null, 2));
}
```

### 10. Performance Benchmark (`benchmark.js`)
Simple performance testing
```javascript
function benchmark(name, fn, iterations = 10000) {
  console.log(`\nBenchmarking: ${name}`);
  
  const startMem = Deno.core.memoryUsage();
  const startTime = Date.now();
  
  for (let i = 0; i < iterations; i++) {
    fn();
  }
  
  const endTime = Date.now();
  const endMem = Deno.core.memoryUsage();
  
  console.log(`  Time: ${endTime - startTime}ms for ${iterations} iterations`);
  console.log(`  Rate: ${Math.round(iterations / (endTime - startTime) * 1000)} ops/sec`);
  console.log(`  Memory delta: ${Math.round((endMem.heapUsed - startMem.heapUsed)/1024)}KB`);
}

// Benchmark different operations
benchmark("Math.sqrt", () => Math.sqrt(Math.random() * 1000));
benchmark("JSON.parse", () => JSON.parse('{"test": true}'));
benchmark("Array.map", () => [1,2,3,4,5].map(x => x * 2));
benchmark("String operations", () => "hello world".toUpperCase().split(" ").join("-"));
benchmark("Type checking", () => {
  const val = [1,2,3];
  Deno.core.isDate(val) || Deno.core.isPromise(val) || Deno.core.isTypedArray(val);
});

console.log("\nFinal memory state:");
const finalMem = Deno.core.memoryUsage();
console.log(`Heap: ${Math.round(finalMem.heapUsed/1024)}KB / ${Math.round(finalMem.heapTotal/1024)}KB`);
```
