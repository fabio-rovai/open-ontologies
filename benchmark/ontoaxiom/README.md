### Ontology Learning with LLMs: A Benchmark Study on Axiom Identification

This repository contains ontologies, results, texts, and prompts that belong to the paper _Ontology Learning with LLMs:
A Benchmark Study on Axiom Identification_.

## Features
- Evaluation of multiple LLM models (GPT-4.1, GPT-4o, Qwen, Llama, Mistral, DeepSeek, etc.)
- Support for different ontology use cases (nordstream, pizza, music, era, foaf, goodrelations, gufo, saref, time)
- Multiple axiom types (subclasses, disjoint, subproperty, domain, range)
- Various prompting strategies (zeroshot, oneshot, fewshot)
- Comprehensive evaluation metrics and scripts (precision, recall, F1 score)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/trustllm.git
cd trustllm
```

2. Create and activate a virtual environment:
```bash
python -m venv venv
# On Windows
venv\Scripts\activate
# On Unix or MacOS
source venv/bin/activate
```

3. Install dependencies:
```bash
pip install -r requirements.txt
```

4. Set up environment variables:
Create a `.env` file in the root directory with your API keys and configuration.

## Usage

### Running Experiments
To run experiments with default configuration:
```bash
python src/run_experiments.py
```

Put in the right API address and key in axiom_extraction.py.
You can modify the configuration in `src/run_experiments.py` to customize:
- Use cases
- Models
- Methods (direct, separate)
- Shot types (zeroshot, oneshot, fewshot)
- Number of runs

### Evaluating Results
To evaluate the results of experiments either generate your own results or unzip the results in the data/results folder.

```bash
python src/run_evaluation.py
```

This will generate CSV files with evaluation metrics in the `data/results` directory.

### Visualizing Results
The project includes scripts for visualizing results in the `src/ontology_evaluation/visualize_results.py` file.

## Project Structure
```
trustllm/
├── data/                      # Data files
│   ├── ontoaxiom/             # OntoAxiom benchmark
│   └── results/               # Experiment results and visualizations
├── src/                       # Source code
│   ├── ontology_analysis/     # Ontology analysis tools
│   ├── ontology_evaluation/   # Evaluation scripts
│   ├── axiom_extraction.py    # Axiom extraction from LLMs
│   ├── config.py              # Configuration settings
│   ├── main.py                # Main application logic
│   ├── prompts.py             # LLM prompts
│   ├── run_evaluation.py      # Evaluation script
│   └── run_experiments.py     # Experiment runner
├── requirements.txt           # Project dependencies
└── README.md                  # This file
```

## Experiment Configuration
The experiments can be configured with the following parameters:
- **Use cases**: Different ontology domains (nordstream, pizza, music, etc.)
- **Models**: Various LLMs (GPT-4.1, GPT-4o, Qwen, Llama, etc.)
- **Methods**: Approaches for axiom extraction (direct, separate (AbA in the paper))
- **Shot types**: Prompting strategies (zeroshot, oneshot, fewshot)
- **Runs**: Number of experiment repetitions for statistical significance

## Evaluation Metrics
The project evaluates LLM performance using:
- **Precision**: Ratio of correctly identified axioms to all identified axioms
- **Recall**: Ratio of correctly identified axioms to all ground truth axioms
- **F1 Score**: Harmonic mean of precision and recall

## License
This work is released under Apache 2.0, for more information see the LICENSE.

## Contributors
Anonymous

## Acknowledgements
Anonymous