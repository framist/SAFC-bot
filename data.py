import sqlite3
import json
import hashlib

"""
原始数据的处理、建立数据库脚本文件。请保留以供未来参考

使用 sqlite - 关系型数据库，弱类型

TODO 转换为严格的关系型数据库，目前为了敏捷开发，使用 ~2NF

原始 json 数据格式：
school_cate: 学校类别
university: 学校
department: 学院
supervisor: 导师

description: 评价
date: 日期
+ other_info: 其他信息


_表示后续可变

【客体表】objects
_学校类别 < 学校 < 学院 < 导师 - _日期 - _信息 - object (key)
          | 包含学院本身 self 下同

object：
sha256( 学校 | 学院 | 导师 )[:8]

【评价表】comments
object < 评价 - 日期 - _来源分类 - _评价类型 - 发布人签名 - 评价 id (key)

来源分类：admin, urfire, telegram...
评价类型：nest（评价的评价）, teacher, course, student, unity, info（wiki_like） ...
评价 id = sha256( object | 评价 | 日期 )[:8] 注意，这个也包含去重的性质
发布人签名 可为空 = sha256( 评价 id | sha256(salt + 发布人一次性密语).hex )
salt: SAFC_salt

TODO 区块链？
"""

SAFC_ASLT = b'SAFC_salt'

def json_to_sqlite():
    # 读取 JSON 文件
    with open('./comments_data.json', 'r') as file:
        data = json.load(file)

    # 打印 data 的键值
    print(data[0].keys())
    # ['school_cate', 'university', 'department', 'supervisor', 'rate', 'description', 'date', 'counts']

    # 连接到数据库（如果不存在则创建）
    with sqlite3.connect('db.sqlite') as conn:
        cursor = conn.cursor()

        # 创建 objects 表格
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS objects (
                school_cate TEXT NOT NULL,
                university TEXT NOT NULL,
                department TEXT NOT NULL,
                supervisor TEXT NOT NULL,
                date TEXT NOT NULL,
                info TEXT,
                object TEXT NOT NULL,
                PRIMARY KEY (object)
            )  
        """)
        
        # 创建 comments 表格
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS comments (
                object TEXT NOT NULL,
                description TEXT NOT NULL,
                date TEXT NOT NULL,
                source_cate TEXT NOT NULL,
                type TEXT NOT NULL,
                author_sign TEXT,
                id TEXT NOT NULL,
                PRIMARY KEY (id)
            )
        """)
        

        # 遍历数据并插入到数据库
        for item in data:
            object_key = hashlib.sha256(f"{item['university']}{item['department']}{item['supervisor']}".encode()).hexdigest()[:16]
            cursor.execute("""INSERT OR IGNORE INTO objects 
                (school_cate, university, department, supervisor, date, object)
                VALUES (?, ?, ?, ?, ?, ?)""",
                (item['school_cate'], 
                 item['university'], 
                 item['department'], 
                 item['supervisor'], 
                 '2022-05',  # urfire 数据最后的评价日期
                 object_key)
            )
            comment_id = hashlib.sha256(f"{object_key}{item['description']}{item['date']}".encode()).hexdigest()[:16]
            cursor.execute("""INSERT OR IGNORE INTO comments
                (object, description, date, source_cate, type, id)
                VALUES (?, ?, ?, ?, ?, ?)""",
                (object_key,
                 item['description'],
                 item['date'],
                 'urfire',
                 'teacher',
                 comment_id
                 )
            )

        # 提交更改
        conn.commit()

json_to_sqlite()

# 信息展示
with sqlite3.connect('db.sqlite') as conn:
    cursor = conn.cursor()
    # 所有的学校类别
    cursor.execute("SELECT DISTINCT school_cate FROM objects")
    
    cursor.execute("SELECT DISTINCT department FROM objects WHERE school_cate=? AND university=?",
                ("985", "清华大学"))
    print(cursor.fetchall())