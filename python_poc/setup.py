from setuptools import setup, find_packages

with open("requirements.txt") as f:
    requirements = f.read().splitlines()

setup(
    name="sql-tdg",
    version="0.0.1",
    description="A SQL query to test data generator.",
    author="Philip Sjöberg",
    author_email="phdah.sjoberg@gmail.com",
    url="https://github.com/phdah/sql-tdg",
    packages=find_packages(include=["sql_tdg", "sql_tdg.*"]),
    install_requires=requirements,
    classifiers=[
        "Programming Language :: Python :: 3.12",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.12",
)
