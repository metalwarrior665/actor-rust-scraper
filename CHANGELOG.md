#### 2021-03-15
- Updated internals and libraries. The scraper should be even more efficient now.

#### 2020-04-06
- Added `max_request_retries` field and retrying failed requests in general.

#### 2020-04-04
- Removed sync mode and `run_async` option. Everything is async now.
- Added `max_concurrency` field. This fixes all memory problems with previous async implementation.

#### 2020-02-09
- Added support of async scraping. Can be turned on with `"run_async": true`.
- Added buffering of results before pushing into dataset (to not overwhelm Apify API). Can be changed via `push_data_size`.