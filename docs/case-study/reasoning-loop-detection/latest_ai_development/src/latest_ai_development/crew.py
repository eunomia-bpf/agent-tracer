# src/my_project/crew.py
from crewai import Agent, Crew, Process, Task
from crewai.project import CrewBase, agent, crew, task
from crewai_tools import SerperDevTool
# from latest_ai_development.tools.MarkdownTools import markdown_validation_tool
from langchain_openai import ChatOpenAI

@CrewBase
class LatestAiDevelopmentCrew():
	"""LatestTopicResearch crew"""

	@agent
	def researcher(self) -> Agent:
		return Agent(
			config=self.agents_config['researcher'],
			verbose=True,
			tools=[SerperDevTool()]
		)

	@agent
	def reporting_analyst(self) -> Agent:
		return Agent(
			config=self.agents_config['reporting_analyst'],
			verbose=True,
			tools=[SerperDevTool()]
		)

	# @agent
	# def tech_writer(self) -> Agent:
	# 	return Agent(
	# 		config=self.agents_config['tech_writer'],
	# 		# tools=[markdown_validation_tool],
	# 		verbose=True
	# 	)

	@task
	def research_task(self) -> Task:
		return Task(
			config=self.tasks_config['research_task'],
		)

	# @task
	# def summary_task(self) -> Task:
	# 	return Task(
	# 		config=self.tasks_config['summary_task'],
	# 		output_file='summary.md'
	# 	)

	@task
	def reporting_task(self) -> Task:
		return Task(
			context=[self.research_task()],
			config=self.tasks_config['reporting_task'],
		)

	# @task
	# def write_tech_article_task(self) -> Task:
	# 	return Task(
	# 		config=self.tasks_config['blog_task'],
	# 		output_file='blog.md'
	# 	)

	@crew
	def crew(self) -> Crew:
		"""Creates the LatestTopicResearch crew"""
		return Crew(
			agents=self.agents, # Automatically created by the @agent decorator
			tasks=self.tasks, # Automatically created by the @task decorator
			process=Process.sequential,
			verbose=True,
			planning=True,
			planning_llm=ChatOpenAI(model="gpt-4o"),
			# process=Process.hierarchical, # In case you wanna use that instead https://docs.crewai.com/how-to
			memory=True,
    		# manager_llm=ChatOpenAI(model="gpt-4o")		
		)