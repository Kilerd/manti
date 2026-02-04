#!/usr/bin/env python3
"""
Manti Admin CLI - 管理用户和 API Keys
"""

import requests
import json
import sys
import getpass
from typing import Optional
import argparse
from datetime import datetime

BASE_URL = "http://localhost:8080"

class MantiAdminCLI:
    def __init__(self, base_url: str = BASE_URL):
        self.base_url = base_url
        self.token: Optional[str] = None

    def register_user(self, email: str, username: str, password: str):
        """注册新用户"""
        response = requests.post(
            f"{self.base_url}/auth/register",
            json={
                "email": email,
                "username": username,
                "password": password
            }
        )

        if response.status_code == 200:
            user_info = response.json()
            print(f"✅ 用户注册成功: {user_info['username']} ({user_info['email']})")
            return True
        elif response.status_code == 409:
            print("❌ 用户已存在")
            return False
        else:
            print(f"❌ 注册失败: {response.status_code}")
            return False

    def login(self, email: str, password: str):
        """用户登录"""
        response = requests.post(
            f"{self.base_url}/auth/login",
            json={
                "email": email,
                "password": password
            }
        )

        if response.status_code == 200:
            login_data = response.json()
            self.token = login_data['token']
            print(f"✅ 登录成功")
            print(f"   用户: {login_data['user']['username']}")
            print(f"   邮箱: {login_data['user']['email']}")
            return True
        else:
            print(f"❌ 登录失败: {response.status_code}")
            return False

    def create_api_key(self, name: str, expires_days: int = 365,
                      rate_limit: int = 60, models: Optional[list] = None):
        """创建 API Key"""
        if not self.token:
            print("❌ 请先登录")
            return None

        payload = {
            "name": name,
            "expires_in_days": expires_days,
            "rate_limit_rpm": rate_limit
        }

        if models:
            payload["allowed_models"] = models

        response = requests.post(
            f"{self.base_url}/api-keys",
            headers={"Authorization": f"Bearer {self.token}"},
            json=payload
        )

        if response.status_code == 200:
            key_data = response.json()
            print(f"✅ API Key 创建成功")
            print(f"   名称: {key_data['name']}")
            print(f"   密钥: {key_data['key']}")
            print(f"   前缀: {key_data['prefix']}")
            print(f"   过期: {key_data.get('expires_at', '永不过期')}")
            print("\n⚠️  请保存好这个密钥，它只会显示一次！")
            return key_data['key']
        else:
            print(f"❌ API Key 创建失败: {response.status_code}")
            return None

    def list_api_keys(self):
        """列出所有 API Keys"""
        if not self.token:
            print("❌ 请先登录")
            return

        response = requests.get(
            f"{self.base_url}/api-keys",
            headers={"Authorization": f"Bearer {self.token}"}
        )

        if response.status_code == 200:
            keys = response.json()
            print(f"\n📋 API Keys 列表 (共 {len(keys)} 个):")
            print("-" * 60)

            for key in keys:
                status = "✅ 活跃" if key['is_active'] else "❌ 已撤销"
                print(f"  {status} {key['name']} ({key['prefix']}...)")
                print(f"      创建时间: {key['created_at']}")
                if key.get('last_used'):
                    print(f"      最后使用: {key['last_used']}")
                if key.get('expires_at'):
                    print(f"      过期时间: {key['expires_at']}")
                if key.get('rate_limit_rpm'):
                    print(f"      速率限制: {key['rate_limit_rpm']} RPM")
                if key.get('allowed_models'):
                    print(f"      允许模型: {', '.join(key['allowed_models'])}")
                print()
        else:
            print(f"❌ 获取 API Keys 失败: {response.status_code}")

    def test_api_key(self, api_key: str):
        """测试 API Key 是否有效"""
        response = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers={"Authorization": f"Bearer {api_key}"},
            json={
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "user", "content": "Say 'API Key is working!'"}
                ],
                "stream": False,
                "max_tokens": 20
            }
        )

        if response.status_code == 200:
            print("✅ API Key 有效，可以正常使用")
            return True
        else:
            print(f"❌ API Key 测试失败: {response.status_code}")
            if response.text:
                print(f"   错误: {response.text}")
            return False

def main():
    parser = argparse.ArgumentParser(description="Manti LLM Gateway 管理工具")
    parser.add_argument("--url", default=BASE_URL, help="Gateway URL")

    subparsers = parser.add_subparsers(dest="command", help="可用命令")

    # 注册命令
    register_parser = subparsers.add_parser("register", help="注册新用户")
    register_parser.add_argument("email", help="用户邮箱")
    register_parser.add_argument("username", help="用户名")

    # 登录命令
    login_parser = subparsers.add_parser("login", help="用户登录")
    login_parser.add_argument("email", help="用户邮箱")

    # 创建 API Key 命令
    create_key_parser = subparsers.add_parser("create-key", help="创建 API Key")
    create_key_parser.add_argument("name", help="API Key 名称")
    create_key_parser.add_argument("--expires", type=int, default=365, help="过期天数（默认 365）")
    create_key_parser.add_argument("--rate-limit", type=int, default=60, help="速率限制 RPM（默认 60）")
    create_key_parser.add_argument("--models", nargs="+", help="允许的模型列表")

    # 列出 API Keys 命令
    list_keys_parser = subparsers.add_parser("list-keys", help="列出所有 API Keys")

    # 测试 API Key 命令
    test_key_parser = subparsers.add_parser("test-key", help="测试 API Key")
    test_key_parser.add_argument("api_key", help="要测试的 API Key")

    # 快速设置命令
    quick_setup_parser = subparsers.add_parser("quick-setup", help="快速设置（创建用户并生成 API Key）")
    quick_setup_parser.add_argument("email", help="用户邮箱")
    quick_setup_parser.add_argument("username", help="用户名")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    cli = MantiAdminCLI(args.url)

    if args.command == "register":
        password = getpass.getpass("请输入密码: ")
        cli.register_user(args.email, args.username, password)

    elif args.command == "login":
        password = getpass.getpass("请输入密码: ")
        cli.login(args.email, password)

    elif args.command == "create-key":
        # 自动登录
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.create_api_key(
                args.name,
                args.expires,
                args.rate_limit,
                args.models
            )

    elif args.command == "list-keys":
        # 自动登录
        email = input("请输入邮箱进行登录: ")
        password = getpass.getpass("请输入密码: ")

        if cli.login(email, password):
            cli.list_api_keys()

    elif args.command == "test-key":
        cli.test_api_key(args.api_key)

    elif args.command == "quick-setup":
        print("\n🚀 快速设置 Manti LLM Gateway")
        print("-" * 40)

        # 设置密码
        password = getpass.getpass("请设置密码: ")
        confirm_password = getpass.getpass("请确认密码: ")

        if password != confirm_password:
            print("❌ 密码不匹配")
            return

        # 注册用户
        if cli.register_user(args.email, args.username, password):
            # 登录
            if cli.login(args.email, password):
                # 创建默认 API Key
                api_key = cli.create_api_key(
                    "Default API Key",
                    expires_days=365,
                    rate_limit=60
                )

                if api_key:
                    print("\n" + "="*60)
                    print("✅ 设置完成！")
                    print("\n您可以使用以下 API Key 访问服务:")
                    print(f"\n{api_key}")
                    print("\n示例命令:")
                    print(f"""
curl -X POST {args.url}/v1/chat/completions \\
  -H "Authorization: Bearer {api_key}" \\
  -H "Content-Type: application/json" \\
  -d '{{"model": "gpt-4o-mini", "messages": [{{"role": "user", "content": "Hello!"}}]}}'
                    """)

if __name__ == "__main__":
    main()