## Rust Scraper

Speed of light scraping with Rust programming language

**This is super early version for experimentation. Use at your own risk!**

Rust is one of the fastest programming languages out there. In many cases, it matches speed of C. Although JavaScript offer huge flexibility and development speed, we can use Rust to significantly speed up the crawling or reduce costs.

The main speed advantage it has is in HTML parsing where its efficient data structures come ahead of JavaScript.

### Limitations of this actor (will be overcomed in the future)
- This actor only works for scraping pure HTML websites (basically an alternative for [Cheerio Scraper](https://apify.com/apify/cheerio-scraper))
- You can only provide static list of URLs, it cannot enqueue any more.
- It doesn't have a page function, only simplified interface to define what should be scraped.
- It cannot retry retry failed requests (they return `null` for the failed attributes)

### Input
Input in a JSON object with these properties. You can also set it up on Apify platform with a nice UI.
- `startUrls` (array(object)) Array of [request objects](https://sdk.apify.com/docs/api/request#docsNav). At the simplest level a request object looks like this: `{ "url": "http://example.com" }`
- `proxy_settings` (object) Proxy configuration of the actor. By default it uses automatic proxy but you can set it to `None` by passing `{ use-apify_proxy: false }`.
- `extact` (object) Extraction config. This will determine how and what data will be extracted. Check [Data extraction](#data-extraction)

### Data extraction
You should provide extraction configuration object. Such object will define selectors to find on the page, what to extract from those selector and finally names of the fields that the data should be saved as.

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
        "use_apify_proxy": true,
        "apify_proxy_groups": ["SHADER"] // Usually you want to skip specifying groups
    },
    "urls": [
        "https://www.amazon.com/dp/B01CYYU8YW",
        "https://www.amazon.com/dp/B01FXMDA2O",
        "https://www.amazon.com/dp/B00UNT0Y2M"
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
You can run this locally if you have Rust installed. You need to build it before running. If you want to use proxy, don't forget to add your `APIFY_PROXY_PASSWORD`, otherwise you will get nasty error.