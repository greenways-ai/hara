import xml.etree.ElementTree as ET
import os
import sys

def parse_jacoco_report(xml_content):
    """
    Parses the JaCoCo XML report and identifies classes with less than 50% line coverage.

    Args:
        xml_content (str): The content of the jacoco.xml file.

    Returns:
        list: A list of class names with less than 50% line coverage.
    """
    root = ET.fromstring(xml_content)
    classes_for_unit_tests = []

    for package in root.findall('.//package'):
        for clazz in package.findall('.//class'):
            class_name_full = clazz.get('name')
            class_name_java = class_name_full.replace('/', '.')
            
            line_counter = None
            for counter in clazz.findall('counter'):
                if counter.get('type') == 'LINE':
                    line_counter = counter
                    break
            
            if line_counter is not None:
                missed_lines = int(line_counter.get('missed', '0'))
                covered_lines = int(line_counter.get('covered', '0'))
                
                total_lines = missed_lines + covered_lines
                if total_lines > 0:
                    coverage_percentage = (covered_lines / total_lines) * 100
                    if coverage_percentage < 50:
                        classes_for_unit_tests.append(class_name_java)
    return classes_for_unit_tests

def parse_jacoco_methods(xml_content, coverage_threshold=0):
    """
    Parses the JaCoCo XML report and identifies methods with coverage below a given threshold.

    Args:
        xml_content (str): The content of the jacoco.xml file.
        coverage_threshold (int): The maximum percentage of covered methods to be considered uncovered.

    Returns:
        list: A list of dictionaries, each representing an uncovered method.
              Each dictionary contains 'class_name', 'method_name', 'method_desc', 'line_number'.
    """
    root = ET.fromstring(xml_content)
    uncovered_methods = []

    for package in root.findall('.//package'):
        for clazz in package.findall('.//class'):
            class_name_full = clazz.get('name')
            class_name_java = class_name_full.replace('/', '.')

            for method in clazz.findall('.//method'):
                method_name = method.get('name')
                method_desc = method.get('desc')
                line_number = method.get('line')
                
                sys.stderr.write(f"DEBUG: Found Method: {class_name_java}#{method_name}{method_desc} (Line: {line_number})\n")

                method_counter = None
                for counter in method.findall('counter'):
                    sys.stderr.write(f"DEBUG:     Found Counter: type={counter.get('type')}, missed={counter.get('missed')}, covered={counter.get('covered')}\n")
                    if counter.get('type') == 'METHOD':
                        method_counter = counter
                        break
                
                if method_counter is not None:
                    missed_methods = int(method_counter.get('missed', '0'))
                    covered_methods = int(method_counter.get('covered', '0'))

                    total_methods = missed_methods + covered_methods
                    method_coverage_percentage = 0
                    if total_methods > 0:
                        method_coverage_percentage = (covered_methods / total_methods) * 100
                    
                    sys.stderr.write(f"DEBUG: Class: {class_name_java}, Method: {method_name}{method_desc}, Missed: {missed_methods}, Covered: {covered_methods}, Total: {total_methods}, Coverage: {method_coverage_percentage:.2f}%\n")

                    if missed_methods > 0:
                        uncovered_methods.append({
                            'class_name': class_name_java,
                            'method_name': method_name,
                            'method_desc': method_desc,
                            'line_number': line_number
                        })
                sys.stderr.write(f"DEBUG: Finished processing method: {class_name_java}#{method_name}{method_desc}\n")
    return uncovered_methods

def generate_markdown_report(classes, output_file):
    """
    Generates a Markdown report from the list of classes.

    Args:
        classes (list): A list of class names.
        output_file (str): The path to the output Markdown file.
    """
    with open(output_file, 'w') as f:
        f.write("# Classes Requiring Unit Tests (Less than 50% Line Coverage)\n\n")
        if classes:
            f.write("The following classes have less than 50% line coverage and require unit tests:\n\n")
            for clazz in sorted(classes):
                f.write(f"- `{clazz}`\n")
        else:
            f.write("All classes have 50% or more line coverage. Great job!\n")

def generate_method_coverage_report(methods, output_file):
    """
    Generates a Markdown report for methods with low coverage.

    Args:
        methods (list): A list of dictionaries, each representing an uncovered method.
        output_file (str): The path to the output Markdown file.
    """
    with open(output_file, 'w') as f:
        f.write("# Uncovered Static Methods (0% Method Coverage)\n\n")
        if methods:
            f.write("The following static methods have 0% method coverage and require unit tests:\n\n")
            for method in sorted(methods, key=lambda x: (x['class_name'], x['method_name'])):
                f.write(f"- `{method['class_name']}#{method['method_name']}{method['method_desc']}` (Line: {method['line_number']})\n")
        else:
            f.write("All static methods have some method coverage. Great job!\n")

if __name__ == "__main__":
    jacoco_xml_path = "/Users/chris/Development/greenways/hara.lang/target/site/jacoco/jacoco.xml"
    output_md_path_classes = "/Users/chris/Development/greenways/hara.lang/unit_test_action_items.md"
    output_md_path_methods = "/Users/chris/Development/greenways/hara.lang/uncovered_static_methods.md"

    try:
        with open(jacoco_xml_path, 'r') as f:
            xml_content = f.read()
    except FileNotFoundError:
        print(f"Error: {jacoco_xml_path} not found. Please ensure JaCoCo report is generated.")
        exit(1)
    
    # Generate report for classes with low line coverage
    classes_to_test = parse_jacoco_report(xml_content)
    generate_markdown_report(classes_to_test, output_md_path_classes)
    print(f"Generated class coverage report: {output_md_path_classes}")

    # Generate report for methods with low method coverage
    # Note: JaCoCo XML does not directly indicate 'static'. We'll assume all methods
    # with 0% coverage are candidates for now, and filter for static later if needed.
    uncovered_methods = parse_jacoco_methods(xml_content, coverage_threshold=0)
    generate_method_coverage_report(uncovered_methods, output_md_path_methods)
    print(f"Generated method coverage report: {output_md_path_methods}")