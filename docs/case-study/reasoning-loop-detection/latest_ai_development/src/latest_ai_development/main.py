#!/usr/bin/env python
# src/my_project/main.py
import os
import sys
from latest_ai_development.crew import LatestAiDevelopmentCrew

def run():
    """
    Run the crew.
    """
    # read the context from input.txt
    with open('input.txt', 'r') as file:
        context = file.read()
    topic = os.getenv('TOPIC', 'classical system research paper')
    if not context:
        raise Exception('No context provided')
    if not topic:
        raise Exception('No topic provided')
    inputs = {
        'topic': topic,
        'context': context
    }
    LatestAiDevelopmentCrew().crew().kickoff(inputs=inputs)