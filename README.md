<!-- toc start -->
## Rust Scraper

<!-- toc end -->
**This is super early version for experimentation. Use at your own risk!**

Speed of light scraping with Rust programming language. This is meant to be a faster (but less flexible) version of Apify's JavaScript based [Cheerio Scraper](https://apify.com/apify/cheerio-scraper).

Rust is one of the fastest programming languages out there. In many cases, it matches the speed of C. Although JavaScript offers huge flexibility and development speed, we can use Rust to significantly speed up the crawling and/or reduce costs. Rust scraper is both faster and requires less memory.

### Changelog
You can read about fixes and updates in the detailed [changelog file](https://github.com/metalwarrior665/actor-rust-scraper/blob/master/CHANGELOG.md).

### WARNING!!! Don't DDOS a website!
Because this scraper is so fast, you can easily take a website down. This matters especially if you scrape **more than few hundred URLs** and use the **async** scraping mode.
How to prevent that:
- Set reasonable `max_concurrency` input field. You can still scrape very fast and with tiny memory footprint if you set it below `10`.
- If you want to set high `max_concurrency`, only scrape large websites that can handle a load of 1000 requests/second and more.
- Use large pool of proxies so they are not immediately banned.

**If we see you abusing this scraper for attacks on Apify platform, your account can be banned**.

### Why it is faster/cheaper than Cheerio Scraper?
Rust is statically typed language compiled directly into machine code. Because of this, it can optimize the code into the most efficient structures and algorithms. Of course, it is also job of the programmer to write the code efficiently so we expect further improvements for this scraper.

- HTML parsing is about 3 times faster because of efficient data structures.
- HTTP requests are also faster.
- Very efficient async implementation with futures (promises in JS).
- Can offload work to other CPU cores via system threads, scales to full actor memory (native JS doesn't support user created threads).
- Much lower memory usage due to efficient data structures.

### Limitations of this actor (some will be solved in the future)
- This actor only works for scraping pure HTML websites (basically an alternative for [Cheerio Scraper](https://apify.com/apify/cheerio-scraper))
- You can only provide static list of URLs, it cannot enqueue any more.
- It doesn't have a page function, only simplified interface (`extract` object) to define what should be scraped.
- Retries are very simplistic
- It doesn't have a sophisticated concurrency system. It will grow to `max_concurrency` unless CPU gets overwhelmed.

### Input
Input is a JSON object with the properties below explained in detail on the [Apify Store page](https://apify.com/lukaskrivka/rust-scraper/input-schema). You can also set it up on Apify platform with a nice UI.

### Data extraction
You need to provide an [extraction configuration object](https://apify.com/lukaskrivka/rust-scraper/input-schema#extract). This object defines selectors to find on the page, what to extract from those selector and finally names of the fields that the data should be saved as.

`extract` (array) is an array of objects where each object has:
- `field_name` (string) Defines to which field will the data be assigned in your resulting dataset
- `selector` (string) CSS selector to find the data to extract
- `extract_type` (object) What to extract
    - `type` (string) Can be `Text` or `Attribute`
    - `content` (string) Provide only when `type` is `Attribute`

Full INPUT example:
```
{
    "proxy_settings": {
        "useApifyProxy": true,
        "apifyProxyGroups": ["SHADER"]
    },
    "urls": [
        { "url": "https://www.amazon.com/dp/B01CYYU8YW" },
        { "url": "https://www.amazon.com/dp/B01FXMDA2O" },
        { "url": "https://www.amazon.com/dp/B00UNT0Y2M" }
    ],
    "extract": [
        {
            "field_name": "title",
            "selector": "#productTitle",
            "extract_type": {
                "type": "Text"
            }
        },
        {
            "field_name": "customer_reviews",
            "selector": "#acrCustomerReviewText",
            "extract_type": {
                "type": "Text"
            }
        },
        {
            "field_name": "seller_link",
            "selector": "#bylineInfo",
            "extract_type": {
                "type": "Attribute",
                "content": "href"
            }
        }    
    ]
}
```

Output example in JSON (This depends purely on your `extract` config)
```
[
    {
        "seller_link":"/Propack/b/ref=bl_dp_s_web_3039360011?ie=UTF8&node=3039360011&field-lbr_brands_browse-bin=Propack","customer_reviews":"208 customer reviews",
        "title":"Propack Twist - Tie Gallon Size Storage Bags 100 Bags Pack Of 4"
    },
    {
        "byline_link":"/Ziploc/b/ref=bl_dp_s_web_2581449011?ie=UTF8&node=2581449011&field-lbr_brands_browse-bin=Ziploc","customers":"561 customer reviews",
        "title":"Ziploc Gallon Slider Storage Bags, 96 Count"
    },
    {
        "byline_link":"/Reynolds/b/ref=bl_dp_s_web_2599601011?ie=UTF8&node=2599601011&field-lbr_brands_browse-bin=Reynolds","customers":"456 customer reviews",
        "title":"Reynolds Wrap Aluminum Foil (200 Square Foot Roll)"
    }
]
```
### Local usage
You can run this locally if you have Rust installed. You need to build it before running. If you want to use Apify Proxy, don't forget to add your `APIFY_PROXY_PASSWORD` into the environment, otherwise you will get a nasty error.