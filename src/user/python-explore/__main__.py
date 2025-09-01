#!/usr/bin/env python3

import sys
import os

def main():
    print("Python Installation Explorer")
    print(f"Python executable: {sys.executable}")
    print(f"Python version: {sys.version}")
    
    # Show Python path (where modules are searched)
    print("\n--- Python Module Search Paths ---")
    for i, path in enumerate(sys.path):
        print(f"{i+1}. {path}")
    
    # List some standard library modules that should be available
    standard_modules = [
        'os', 'sys', 'time', 'random', 'json', 'math', 'datetime', 
        'collections', 'itertools', 'functools', 'urllib', 'http',
        're', 'string', 'io', 'pathlib', 'subprocess'
    ]
    
    print("\n--- Standard Library Module Check ---")
    available = []
    missing = []
    
    for module_name in standard_modules:
        try:
            __import__(module_name)
            available.append(module_name)
            print(f"✓ {module_name}")
        except ImportError as e:
            missing.append(module_name)
            print(f"✗ {module_name} - {e}")
    
    print(f"\nSummary: {len(available)} available, {len(missing)} missing")
    
    # Try to peek at the file system structure
    print("\n--- File System Exploration ---")
    try:
        # Look for Python library directory
        python_lib_paths = [path for path in sys.path if 'lib/python' in path]
        if python_lib_paths:
            lib_path = python_lib_paths[0]
            print(f"Python library path: {lib_path}")
            try:
                contents = os.listdir(lib_path)
                print(f"Found {len(contents)} items in library directory")
                # Show first 10 items
                for item in sorted(contents)[:10]:
                    print(f"  - {item}")
                if len(contents) > 10:
                    print(f"  ... and {len(contents) - 10} more items")
            except Exception as e:
                print(f"Can't list library contents: {e}")
    except Exception as e:
        print(f"Error exploring filesystem: {e}")

if __name__ == "__main__":
    main()