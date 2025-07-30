from typing import List, Tuple
from crewai_tools import BaseTool


class MarkdownCheckTool(BaseTool):
    # name: str = "Name of my tool"
    # description: str = (
    #     "Clear description for what this tool is useful for, you agent will need this information to use it."
    # )
    name: str = "Markdown Link Checker"
    description: str = (
        "This tool checks the validity of all Markdown links in the given text."
    )

    def _run(self, argument: str) -> str:
        # Implementation goes here
        res = self.check_markdown_links(argument)
        # convert the results to a string
        return "\n".join([f"{url}: {status}" for url, status in res])


    def check_markdown_links(markdown_text: str, timeout: int = 5) -> List[Tuple[str, str]]:
        """
        Extracts all Markdown links from the given text and checks their validity.

        Parameters:
            markdown_text (str): The Markdown content as a string.
            timeout (int): The timeout for HTTP requests in seconds. Default is 5 seconds.

        Returns:
            List[Tuple[str, str]]: A list of tuples where each tuple contains the URL and its status.
                                Status is 'OK' for successful requests or the error message for failures.
        """
        # Regular expression to find Markdown links: [text](URL)
        link_pattern = re.compile(r'\[([^\]]+)\]\((https?://[^\s)]+)\)')
        links = link_pattern.findall(markdown_text)

        results = []

        for text, url in links:
            print(f"Checking link: {url}")
            try:
                response = requests.head(url, allow_redirects=True, timeout=timeout)
                if response.status_code == 200:
                    status = 'OK'
                else:
                    status = f'Error {response.status_code}'
            except requests.exceptions.RequestException as e:
                status = f'Failed: {e}'
            
            results.append((url, status))
        
        return results
