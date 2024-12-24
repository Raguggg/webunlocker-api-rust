import requests
from concurrent.futures import ThreadPoolExecutor

# Define the function that makes the HTTP request
def fetch_page(page_number):
    url = "http://localhost:5000/request"
    try:
        response = requests.get(url,params={
            "url":f"https://www.property.com.au/search/?locations=Southport,+QLD+4215&pageNumber={page_number}"
        })
        if response.status_code == 200:
            print(f"Page {page_number} fetched successfully")
        else:
            print(f"Page {page_number} failed with status code {response.status_code}")
    except requests.RequestException as e:
        print(f"Error fetching page {page_number}: {e}")

# Set up a thread pool to execute the requests
def main():
    # Create a ThreadPoolExecutor to run tasks in parallel
    with ThreadPoolExecutor(max_workers=55) as executor:
        # Generate a list of page numbers (from 1 to 54)
        page_numbers = range(1, 55)
        
        # Execute the fetch_page function for each page number
        executor.map(fetch_page, page_numbers)

if __name__ == "__main__":
    import time
    start  = time.time()
    main()
    end  = time.time()
    print(end-start)
