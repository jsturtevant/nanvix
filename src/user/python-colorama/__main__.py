#!/usr/bin/env python3

def main():
    print("Testing colorama import...")
    
    try:
        from colorama import Fore, Back, Style, init
        print("✓ colorama imported successfully!")
        
        # Initialize colorama
        init()
        
        # Test colored output
        print(f"{Fore.RED}This text is red!{Style.RESET_ALL}")
        print(f"{Fore.GREEN}This text is green!{Style.RESET_ALL}")
        print(f"{Fore.BLUE}This text is blue!{Style.RESET_ALL}")
        print(f"{Back.YELLOW}{Fore.BLACK}Black text on yellow background!{Style.RESET_ALL}")
        
        print("colorama test complete!")
        
    except ImportError as e:
        print(f"✗ Failed to import colorama: {e}")
    except Exception as e:
        print(f"✗ Error using colorama: {e}")

if __name__ == "__main__":
    main()