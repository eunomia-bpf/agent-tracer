#!/usr/bin/env python3
"""
FilterExpression - A model for parsing and evaluating URL filter expressions

Supports complex expressions with logical operators and different matching conditions.
Now supports both request and response filtering with improved syntax.
"""

from typing import Dict, List, Any, Optional, Union
from urllib.parse import urlparse, parse_qs
import re

class FilterExpression:
    """Parse and evaluate filter expressions for URL exclusion"""
    
    def __init__(self, expression: str, debug: bool = False):
        self.expression = expression.strip()
        self.debug = debug
        self.parsed_expression = self._parse_expression()
        if self.debug:
            print(f"[FILTER DEBUG] Parsed expression '{expression}':")
            print(f"  Structure: {self.parsed_expression}")
        
    def _parse_expression(self) -> Dict[str, Any]:
        """Parse expression into a tree structure with logical operators"""
        if not self.expression:
            return {'type': 'empty'}
            
        # First, split by OR (|) operator (lowest precedence)
        or_parts = [part.strip() for part in self.expression.split('|')]
        
        if len(or_parts) > 1:
            # This is an OR expression
            or_conditions = []
            for part in or_parts:
                or_conditions.append(self._parse_and_expression(part))
            return {
                'type': 'or',
                'conditions': or_conditions
            }
        else:
            # This is an AND expression or a single condition
            return self._parse_and_expression(or_parts[0])
            
    def _parse_and_expression(self, expression: str) -> Dict[str, Any]:
        """Parse AND expression (higher precedence than OR)"""
        and_parts = [part.strip() for part in expression.split('&')]
        
        if len(and_parts) > 1:
            # This is an AND expression
            and_conditions = []
            for part in and_parts:
                and_conditions.append(self._parse_single_condition(part))
            return {
                'type': 'and',
                'conditions': and_conditions
            }
        else:
            # This is a single condition
            return self._parse_single_condition(and_parts[0])
            
    def _parse_single_condition(self, condition: str) -> Dict[str, Any]:
        """Parse a single condition with new dot notation syntax"""
        condition = condition.strip()
        
        if '=' in condition:
            key, value = condition.split('=', 1)
            key = key.strip()
            value = value.strip()
            
            # Check for new dot notation (request.path, response.status_code)
            if '.' in key:
                target_type, field = key.split('.', 1)
                target_type = target_type.lower()
                field = field.strip()
                
                if target_type in ['request', 'req']:
                    return {
                        'type': 'condition',
                        'target': 'request',
                        'condition_type': self._get_request_condition_type(field),
                        'key': field,
                        'value': value
                    }
                elif target_type in ['response', 'resp', 'res']:
                    return {
                        'type': 'condition',
                        'target': 'response',
                        'condition_type': self._get_response_condition_type(field),
                        'key': field,
                        'value': value
                    }
            else:
                # Legacy format - assume request for backward compatibility
                return {
                    'type': 'condition',
                    'target': 'request',
                    'condition_type': self._get_request_condition_type(key),
                    'key': key,
                    'value': value
                }
        else:
            # Simple string containment (backward compatibility) - assume request path
            return {
                'type': 'condition',
                'target': 'request',
                'condition_type': 'path_contains',
                'key': 'path',
                'value': condition
            }
    
    def _get_request_condition_type(self, key: str) -> str:
        """Determine the type of request condition based on key"""
        key = key.lower()
        if key in ['path_prefix', 'path_starts_with']:
            return 'path_prefix'
        elif key in ['path', 'path_exact']:
            return 'path_exact'
        elif key in ['path_contains', 'path_includes']:
            return 'path_contains'
        elif key in ['method', 'verb']:
            return 'method'
        elif key in ['host', 'hostname']:
            return 'host'
        elif key in ['header']:
            return 'request_header'
        elif key in ['body', 'body_contains']:
            return 'request_body'
        else:
            # Assume it's a query parameter
            return 'query_param'
            
    def _get_response_condition_type(self, key: str) -> str:
        """Determine the type of response condition based on key"""
        key = key.lower()
        if key in ['status_code', 'status', 'code']:
            return 'status_code'
        elif key in ['status_text', 'status_message']:
            return 'status_text'
        elif key in ['content_type', 'content-type']:
            return 'content_type'
        elif key in ['server']:
            return 'server'
        elif key in ['header']:
            return 'response_header'
        elif key in ['body', 'body_contains']:
            return 'response_body'
        elif key in ['body_size', 'content_length']:
            return 'body_size'
        else:
            # Assume it's a response header
            return 'response_header'
            
    def evaluate(self, parsed_data: Dict[str, Any]) -> bool:
        """Evaluate the expression against parsed HTTP data"""
        if self.parsed_expression.get('type') == 'empty':
            return False
            
        # Extract basic data
        entry_type = parsed_data.get('type', '')
        
        if self.debug:
            print(f"[FILTER DEBUG] Evaluating {entry_type} entry")
        
        # Evaluate the parsed expression
        result = self._evaluate_expression(self.parsed_expression, parsed_data)
        
        if self.debug:
            print(f"[FILTER DEBUG] Final result: {result}")
            
        return result
        
    def _evaluate_expression(self, expr: Dict[str, Any], data: Dict[str, Any]) -> bool:
        """Evaluate a parsed expression recursively"""
        expr_type = expr.get('type')
        
        if expr_type == 'empty':
            return False
        elif expr_type == 'condition':
            return self._evaluate_condition(expr, data)
        elif expr_type == 'and':
            results = []
            for condition in expr['conditions']:
                result = self._evaluate_expression(condition, data)
                results.append(result)
                if self.debug:
                    print(f"[FILTER DEBUG] AND condition result: {result}")
            final_result = all(results)
            if self.debug:
                print(f"[FILTER DEBUG] AND final result: {final_result} (all of {results})")
            return final_result
        elif expr_type == 'or':
            results = []
            for condition in expr['conditions']:
                result = self._evaluate_expression(condition, data)
                results.append(result)
                if self.debug:
                    print(f"[FILTER DEBUG] OR condition result: {result}")
            final_result = any(results)
            if self.debug:
                print(f"[FILTER DEBUG] OR final result: {final_result} (any of {results})")
            return final_result
        
        return False
        
    def _evaluate_condition(self, condition: Dict[str, Any], data: Dict[str, Any]) -> bool:
        """Evaluate a single condition"""
        target = condition.get('target', 'request')
        condition_type = condition['condition_type']
        key = condition['key']
        value = condition['value']
        
        # Check if the data type matches the target
        data_type = data.get('type', '')
        if target == 'request' and data_type != 'request':
            if self.debug:
                print(f"[FILTER DEBUG] Skipping request condition on {data_type} entry")
            return False
        elif target == 'response' and data_type != 'response':
            if self.debug:
                print(f"[FILTER DEBUG] Skipping response condition on {data_type} entry")
            return False
        
        if target == 'request':
            return self._evaluate_request_condition(condition_type, key, value, data)
        elif target == 'response':
            return self._evaluate_response_condition(condition_type, key, value, data)
        
        return False
        
    def _evaluate_request_condition(self, condition_type: str, key: str, value: str, data: Dict[str, Any]) -> bool:
        """Evaluate request-specific conditions"""
        path = data.get('path', '')
        method = data.get('method', '')
        headers = data.get('headers', {})
        host = headers.get('host', '')
        body = data.get('body', '')
        
        if condition_type == 'path_prefix':
            result = path.startswith(value)
            if self.debug:
                print(f"[FILTER DEBUG]   request.path_prefix: '{path}' starts with '{value}' = {result}")
            return result
        elif condition_type == 'path_exact':
            result = path == value
            if self.debug:
                print(f"[FILTER DEBUG]   request.path_exact: '{path}' == '{value}' = {result}")
            return result
        elif condition_type == 'path_contains':
            result = value in path
            if self.debug:
                print(f"[FILTER DEBUG]   request.path_contains: '{value}' in '{path}' = {result}")
            return result
        elif condition_type == 'method':
            result = method.upper() == value.upper()
            if self.debug:
                print(f"[FILTER DEBUG]   request.method: '{method}' == '{value}' = {result}")
            return result
        elif condition_type == 'host':
            result = host == value
            if self.debug:
                print(f"[FILTER DEBUG]   request.host: '{host}' == '{value}' = {result}")
            return result
        elif condition_type == 'request_header':
            header_value = headers.get(key.lower(), '')
            result = value in header_value
            if self.debug:
                print(f"[FILTER DEBUG]   request.header.{key}: '{header_value}' contains '{value}' = {result}")
            return result
        elif condition_type == 'request_body':
            result = value in body
            if self.debug:
                print(f"[FILTER DEBUG]   request.body_contains: '{value}' in body = {result}")
            return result
        elif condition_type == 'query_param':
            # Parse query parameters from path
            query_params = {}
            if '?' in path:
                parsed_url = urlparse(path)
                query_params = parse_qs(parsed_url.query)
                # Flatten single-value parameters
                for param_key, values in query_params.items():
                    if len(values) == 1:
                        query_params[param_key] = values[0]
            
            if key in query_params:
                param_value = query_params[key]
                result = str(param_value) == value
                if self.debug:
                    print(f"[FILTER DEBUG]   request.query_param: '{key}={param_value}' == '{value}' = {result}")
                return result
            else:
                if self.debug:
                    print(f"[FILTER DEBUG]   request.query_param: '{key}' not found in query params")
                return False
                
        return False
        
    def _evaluate_response_condition(self, condition_type: str, key: str, value: str, data: Dict[str, Any]) -> bool:
        """Evaluate response-specific conditions"""
        status_code = data.get('status_code', 0)
        status_text = data.get('status_text', '')
        headers = data.get('headers', {})
        body = data.get('body', '')
        
        if condition_type == 'status_code':
            try:
                target_code = int(value)
                result = status_code == target_code
                if self.debug:
                    print(f"[FILTER DEBUG]   response.status_code: {status_code} == {target_code} = {result}")
                return result
            except ValueError:
                if self.debug:
                    print(f"[FILTER DEBUG]   response.status_code: invalid status code '{value}'")
                return False
        elif condition_type == 'status_text':
            result = value.lower() in status_text.lower()
            if self.debug:
                print(f"[FILTER DEBUG]   response.status_text: '{value}' in '{status_text}' = {result}")
            return result
        elif condition_type == 'content_type':
            content_type = headers.get('content-type', '')
            result = value in content_type
            if self.debug:
                print(f"[FILTER DEBUG]   response.content_type: '{value}' in '{content_type}' = {result}")
            return result
        elif condition_type == 'server':
            server = headers.get('server', '')
            result = value in server
            if self.debug:
                print(f"[FILTER DEBUG]   response.server: '{value}' in '{server}' = {result}")
            return result
        elif condition_type == 'response_header':
            header_value = headers.get(key.lower(), '')
            result = value in header_value
            if self.debug:
                print(f"[FILTER DEBUG]   response.header.{key}: '{header_value}' contains '{value}' = {result}")
            return result
        elif condition_type == 'response_body':
            result = value in body
            if self.debug:
                print(f"[FILTER DEBUG]   response.body_contains: '{value}' in body = {result}")
            return result
        elif condition_type == 'body_size':
            try:
                target_size = int(value)
                body_size = len(body) if body else 0
                result = body_size >= target_size
                if self.debug:
                    print(f"[FILTER DEBUG]   response.body_size: {body_size} >= {target_size} = {result}")
                return result
            except ValueError:
                if self.debug:
                    print(f"[FILTER DEBUG]   response.body_size: invalid size '{value}'")
                return False
                
        return False
        
    def __str__(self) -> str:
        """String representation of the filter expression"""
        return f"FilterExpression('{self.expression}')"
        
    def __repr__(self) -> str:
        """Detailed representation of the filter expression"""
        return f"FilterExpression(expression='{self.expression}', parsed={self.parsed_expression})"

def test_filter_expression():
    """Test function for FilterExpression"""
    # Test data
    test_request = {
        'type': 'request',
        'method': 'GET',
        'path': '/v1/rgstr?code=202&beta=true',
        'headers': {'host': 'api.example.com', 'user-agent': 'test-client'},
        'body': 'test request body'
    }
    
    test_response = {
        'type': 'response',
        'status_code': 200,
        'status_text': 'OK',
        'headers': {'content-type': 'application/json', 'server': 'nginx/1.18'},
        'body': 'test response body'
    }
    
    # Test cases with new syntax
    test_cases = [
        # Request filters
        'request.path_prefix=/v1/rgstr',
        'request.method=GET',
        'request.host=api.example.com',
        'request.body_contains=test',
        'code=202',  # Legacy query param
        
        # Response filters
        'response.status_code=200',
        'response.status_text=OK',
        'response.content_type=application/json',
        'response.server=nginx',
        'response.body_contains=test',
        
        # Mixed conditions
        'request.path_prefix=/v1/rgstr & response.status_code=200',
        'request.method=GET | response.status_code=404',
        'request.host=api.example.com | response.server=nginx',
        
        # Legacy backward compatibility
        'path_prefix=/v1/rgstr',
        'method=GET',
        '/v1/rgstr'
    ]
    
    print("Testing FilterExpression with new syntax:")
    
    for data, data_name in [(test_request, "REQUEST"), (test_response, "RESPONSE")]:
        print(f"\n=== Testing {data_name} ===")
        print(f"Data: {data['type']} {data.get('method', '')}{data.get('path', '')} {data.get('status_code', '')}")
        
        for expression in test_cases:
            print(f"  Expression: '{expression}'")
            filter_expr = FilterExpression(expression, debug=False)
            result = filter_expr.evaluate(data)
            print(f"    Result: {result}")
        print("-" * 60)

if __name__ == '__main__':
    test_filter_expression() 