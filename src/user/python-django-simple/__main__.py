#!/usr/bin/env python3

def main():
    print("Testing Django import and basic functionality...")
    
    try:
        import django
        print(f"✓ Django imported! Version: {django.get_version()}")
        
        # Test Django settings
        from django.conf import settings
        
        # Configure minimal settings
        settings.configure(
            DEBUG=True,
            SECRET_KEY='test-secret-key-for-nanvix',
            INSTALLED_APPS=[
                'django.contrib.contenttypes',
                'django.contrib.auth',
            ],
            DATABASES={
                'default': {
                    'ENGINE': 'django.db.backends.sqlite3',
                    'NAME': ':memory:',
                }
            }
        )
        
        print("✓ Django settings configured")
        
        # Test Django setup
        django.setup()
        print("✓ Django setup complete")
        
        # Create a simple view-like function
        from django.http import HttpResponse
        from django.template import Template, Context
        
        template = Template("Hello from Django in Nanvix! Version: {{ version }}")
        context = Context({"version": django.get_version()})
        response_content = template.render(context)
        
        print(f"✓ Template rendered: {response_content}")
        
        print("Django basic test complete!")
        
    except ImportError as e:
        print(f"✗ Failed to import Django: {e}")
    except Exception as e:
        print(f"✗ Error with Django: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()