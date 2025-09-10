#!/usr/bin/env python3

def main():
    print("Testing Flask with app.run()...")
    
    try:
        from flask import Flask, jsonify
        print("✓ Flask imported successfully!")
        
        # Create a simple Flask app
        app = Flask(__name__)
        
        @app.route('/')
        def hello():
            return jsonify({
                "message": "Hello from Flask in Nanvix!",
                "status": "success"
            })
        
        @app.route('/test')
        def test():
            return jsonify({
                "message": "Test endpoint working",
                "framework": "Flask",
                "environment": "Nanvix"
            })
        
        print("✓ Flask app created with routes")
        
        # Try to run the server
        print("Attempting to start Flask development server...")
        print("This will try to bind to localhost:5000")
        
        # Run with minimal configuration
        app.run(
            host='127.0.0.1',  # localhost only
            port=5000,
            debug=False,       # disable debug mode (fewer file operations)
            threaded=False,    # disable threading (simpler)
            use_reloader=False # disable auto-reloader (fewer file watches)
        )
        
    except ImportError as e:
        print(f"✗ Failed to import Flask: {e}")
    except Exception as e:
        print(f"✗ Error with Flask server: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()