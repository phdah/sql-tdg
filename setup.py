from setuptools import setup, find_packages

setup(
    name="sql-tdg",  # Replace with your project's name
    version="0.0.1",  # Initial version
    description="A SQL query to test data generator.",
    author="Philip Sjöberg",
    author_email="phdah.sjoberg@gmail.com",
    url="https://github.com/phdah/sql-tdg",  # Replace with your project's URL
    packages=find_packages(include=["sql-tdg"]),
    install_requires=[
        "z3-solver==4.13.4.0",
        "numpy==2.2.3",
        "typing-extensions==4.12.2",
    ],
    classifiers=[
        "Programming Language :: Python :: 3.12",
        "License :: OSI Approved :: MIT License",  # Replace with your chosen license
        "Operating System :: OS Independent",
    ],
    python_requires=">=3.12",
)
