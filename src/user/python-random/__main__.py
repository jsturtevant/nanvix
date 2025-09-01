#!/usr/bin/env python3

import random
import time

def setup_random():
    """Initialize random with time-based seed for better randomness in Nanvix"""
    seed = int(time.time() * 1000000)  # microseconds for more granularity
    random.seed(seed)
    return seed

def main():
    print("Practical Random Example for Nanvix")
    
    # Always seed at the start of your app
    seed_used = setup_random()
    print(f"Using seed: {seed_used}")
    
    # Now generate actually random numbers
    print(f"Random integer (1-100): {random.randint(1, 100)}")
    print(f"Random float: {random.random()}")
    
    # Simulate a simple game
    print("\n--- Simple Dice Game ---")
    player_roll = random.randint(1, 6)
    computer_roll = random.randint(1, 6)
    
    print(f"You rolled: {player_roll}")
    print(f"Computer rolled: {computer_roll}")
    
    if player_roll > computer_roll:
        print("You win! 🎉")
    elif player_roll < computer_roll:
        print("Computer wins! 🤖")
    else:
        print("It's a tie! 🤝")

if __name__ == "__main__":
    main()