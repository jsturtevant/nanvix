#!/usr/bin/env python3

def main():
    print("Testing Flask import and basic functionality...")
    
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
        print("✓ Flask basic test complete!")
        
        # Note: We're not calling app.run() since that would try to start a server
        # which might hit the same fd limits as Django
        
    except ImportError as e:
        print(f"✗ Failed to import Flask: {e}")
    except Exception as e:
        print(f"✗ Error with Flask: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()