#!/usr/bin/env python3

import requests
import json
import time
from datetime import datetime, timedelta
import argparse

def fetch_binance_historical_data(symbol, days):
    """Fetch historical price data from Binance API"""
    
    # Calculate start and end timestamps (Binance uses milliseconds)
    end_time = datetime.now()
    start_time = end_time - timedelta(days=days)
    
    start_ms = int(start_time.timestamp() * 1000)
    end_ms = int(end_time.timestamp() * 1000)
    
    print(f"Fetching data from {start_time} to {end_time}")
    print(f"Total hours needed: {days * 24}")
    
    url = "https://api.binance.com/api/v3/klines"
    
    all_data = []
    current_start_ms = start_ms
    
    # Binance limit is 1000 candles per request
    max_candles_per_request = 1000
    
    while current_start_ms < end_ms:
        # Calculate end time for this chunk (don't exceed total end time)
        chunk_end_ms = min(current_start_ms + (max_candles_per_request * 60 * 60 * 1000), end_ms)
        
        print(f"Fetching chunk: {datetime.fromtimestamp(current_start_ms/1000)} to {datetime.fromtimestamp(chunk_end_ms/1000)}")
        
        params = {
            'symbol': f'{symbol}USDT',
            'interval': '1h',
            'startTime': current_start_ms,
            'endTime': chunk_end_ms,
            'limit': max_candles_per_request
        }
        
        try:
            response = requests.get(url, params=params)
            response.raise_for_status()
            candles = response.json()
            
            print(f"Received {len(candles)} candles for this chunk")
            
            if not candles:
                print("No more data available")
                break
            
            # Convert Binance format to our format
            for candle in candles:
                timestamp = int(candle[0])  # Open time in milliseconds
                close_price = float(candle[4])  # Close price
                
                price_data = {
                    'timestamp': datetime.fromtimestamp(timestamp / 1000).isoformat() + 'Z',
                    'prices': {
                        symbol: close_price
                    }
                }
                all_data.append(price_data)
            
            # Move to next chunk - start from the last candle's close time + 1 hour
            if candles:
                last_candle_time = int(candles[-1][0])
                current_start_ms = last_candle_time + (60 * 60 * 1000)  # Add 1 hour
            else:
                break
            
            # Rate limiting to be nice to Binance API
            time.sleep(0.1)
            
        except requests.RequestException as e:
            print(f"Request error: {e}")
            break
        except Exception as e:
            print(f"Unexpected error: {e}")
            break
    
    # Sort by timestamp (should already be sorted, but just in case)
    all_data.sort(key=lambda x: x['timestamp'])
    
    # Remove duplicates (in case of overlap)
    unique_data = []
    seen_timestamps = set()
    for item in all_data:
        if item['timestamp'] not in seen_timestamps:
            unique_data.append(item)
            seen_timestamps.add(item['timestamp'])
    
    print(f"Successfully fetched {len(unique_data)} unique data points")
    if unique_data:
        print(f"Date range: {unique_data[0]['timestamp']} to {unique_data[-1]['timestamp']}")
    
    return unique_data

def fetch_fear_greed_data(days):
    """Fetch Fear & Greed index historical data"""
    url = f"https://api.alternative.me/fng/?limit={days}"
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        data = response.json()
        
        fear_greed_data = []
        for item in data['data']:
            fear_greed_data.append({
                'timestamp': datetime.fromtimestamp(int(item['timestamp'])).isoformat() + 'Z',
                'value': int(item['value']),
                'classification': item['value_classification']
            })
        
        # IMPORTANT: Fear & Greed API returns data in reverse order (newest first)
        # We need to reverse it to match BTC data (oldest first)
        fear_greed_data.reverse()
            
        return fear_greed_data
        
    except requests.RequestException as e:
        print(f"Error fetching Fear & Greed data: {e}")
        return []

def main():
    parser = argparse.ArgumentParser(description='Fetch historical data from Binance API')
    parser.add_argument('--days', type=int, default=30, help='Number of days to fetch (default: 30)')
    parser.add_argument('--symbols', nargs='+', default=['BTC'], help='Crypto symbols to fetch (default: BTC)')
    
    args = parser.parse_args()
    
    print(f"Fetching {args.days} days of historical data from Binance for {args.symbols}")
    
    # Fetch price data for each symbol
    for symbol in args.symbols:
        print(f"\nFetching price data for {symbol}...")
        price_data = fetch_binance_historical_data(symbol, args.days)
        
        if price_data:
            filename = f"../backtest-data/{symbol.lower()}_prices_{args.days}d.json"
            with open(filename, 'w') as f:
                json.dump(price_data, f, indent=2)
            print(f"Saved {len(price_data)} price points to {filename}")
        else:
            print(f"No data fetched for {symbol}")
    
    # Fetch Fear & Greed data
    print("\nFetching Fear & Greed index data...")
    fear_greed_data = fetch_fear_greed_data(args.days)
    
    if fear_greed_data:
        filename = f"../backtest-data/fear_greed_{args.days}d.json"
        with open(filename, 'w') as f:
            json.dump(fear_greed_data, f, indent=2)
        print(f"Saved {len(fear_greed_data)} Fear & Greed points to {filename}")
    
    print("\nBinance historical data fetch completed!")

if __name__ == "__main__":
    main()