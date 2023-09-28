# 收集 https://pi-review.com/ 的数据
# 此站没有 robots.txt
# 上次数据获取日期 `UPDATAED`
# autocorrect: false

import pathlib
import re
import threading
from datetime import datetime
import requests
from bs4 import BeautifulSoup
import threading
import sqlite3
import hashlib

DATA_PATH = "./db.sqlite"
UPDATED = '2023-09-26'
# 下次获取的时候增量爬取就行


def _filter_department(department):
    pattern = r'Department:(.*?)Website'
    return re.search(pattern, department)[1]


def _filter_university(university: str):
    return university.split(' - ')[1].strip() if ' - ' in university else university


def _filter_supervisor(supervisor: str):
    pattern = r"\((.*?)\)"
    matches = re.findall(pattern, supervisor)
    return matches[0] if matches else supervisor


def crawler(i):
    if not pathlib.Path(f'./tmp/pi-review_pis_{i}.html').exists():
        print(f"重新下载 {i}")
        download(i)
        if not pathlib.Path(f'./tmp/pi-review_pis_{i}.html').exists():
            print(f"{i} 不存在")
            with open("./tmp/crawler—not-exist.txt", "a") as f:
                f.write(f"{i}\n")
            return
    try:
        _parser(i)
    except Exception as e:
        print(f"{i} : {e}")
        with open("./tmp/crawler—parse-err.txt", "a") as f:
            f.write(f"crawler—parse-err {i}\n")
        return


def _parser(i):
    url = f"https://pi-review.com/pis/{i}"
    print(f"正在解析 {url}")

    text = pathlib.Path(f'./tmp/pi-review_pis_{i}.html').read_text()
    soup = BeautifulSoup(text, 'html.parser')

    # 使用 CSS 选择器定位所需的数据
    supervisor = soup.select_one('h1').get_text(strip=True)
    supervisor = _filter_supervisor(supervisor)

    rating = soup.select_one('.star-rating').get_text(strip=True)

    comment_sum = soup.select_one(
        '.text-muted').get_text(strip=True).split()[0].strip()

    # email = soup.find('span', {'class': 'text-monospace'} ).text.strip()
    # print("邮箱:", email)
    li = soup.find_all('li', class_='my-1')
    university = _filter_university(li[1].a.get_text(strip=True))

    department = li[2].get_text(strip=True)
    department = _filter_department(department)

    website = li[3].get_text(strip=True)[8:].strip()

    print("大学:", university)
    print("院系:", department)
    print("姓名:", supervisor)

    info = f"Personal Website: {website}\n"
    print("info", info)
    print("评价数:", comment_sum)
    # 如果还没有评价且没有既定的分类，就不添加了
    object_id = db_add_object(university, department,
                              supervisor, info, comment_sum != 'no')

    print("评分:", rating)  # 舍弃

    comments = soup.find_all('div', class_='review-card')

    for comment in comments:
        t = comment.select_one('.flask-moment').get_text(strip=True)
        t = datetime.strptime(t, "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d")

        print("=======\n时间：", t)
        print("评价：")
        c = comment.select_one('.raw-markdown').get_text(strip=True)
        c = f"{c}\n\nfrom {url} updated at {UPDATED}"
        print(c)
        object_id_nest = db_add_comment(object_id, c, t )
        
        print("嵌套评价：")
        nests = comment.find_all('li', class_='review-comment-card')
        for j in nests:
            n = j.get_text(strip=True)
            c = n[1:-25]
            c = f"{c}\n\nfrom {url} updated at {UPDATED}"
            
            t = datetime.strptime(n[-20:], "%Y-%m-%dT%H:%M:%SZ").strftime("%Y-%m-%d")
            print("嵌套评价：", c)
            print("时间：", t)
            db_add_comment(object_id_nest, c, t)


def db_add_object(university: str, department: str, supervisor: str, info: str, add: bool):
    # 去重地增加评价客体
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # 如果 object_id 存在就只添加 info
        object_id = hashlib.sha256(
            f"{university}{department}{supervisor}".encode()).hexdigest()[:16]
        cursor.execute("SELECT object FROM objects WHERE object=?",
                       (object_id,)
                       )
        if cursor.fetchone():
            print("object_id 已存在, 替代 info 的内容:", object_id)
            # ! 注意，会替代 info 的内容。因为写这个脚本的时候 info 全为 Null
            cursor.execute("UPDATE objects SET info=? WHERE object=?",
                           (info, object_id)
                           )
            conn.commit()
            return object_id

        # 先看看有没有已分好的 school_cate
        # 指定学校下的学院
        # 如果还没有评价且没有既定的分类，就不添加了
        cursor.execute("SELECT DISTINCT school_cate FROM objects WHERE university=?",
                       (university,))
        c = cursor.fetchall()
        school_cate = c[0][0] if c else '未归类'
        if 'University of California' in university or 'University of Illinois at Urbana' in university or 'Stony Brook University' in university:
            school_cate = 'U.S.'
        if '中山大学' in university:
            school_cate = '985'
        if '中国石油大学' in university or '中国地质大学' in university:
            school_cate = '211'
        if school_cate == '未归类' and not add:
            print(f"跳过添加客体，没有评价且没有既定的分类{university}{department}{supervisor}")
            return object_id

        print("学校分类：", school_cate)

        print("object_id：", object_id)
        cursor.execute("""INSERT INTO objects
            (school_cate, university, department, supervisor, date, info, object)
            VALUES (?, ?, ?, ?, ?, ?, ?)""",
                       (
                           school_cate,
                           university,
                           department,
                           supervisor,
                           datetime.now().strftime("%Y-%m-%d"),
                           info,
                           object_id,
                       )
                       )
        conn.commit()
        print('成功添加客体：', object_id)
        return object_id


def db_add_comment(object_id, comment, date):
    """ 增加评价 返回评价 id"""
    comment_id = hashlib.sha256(
        f"{object_id}{comment}{date}".encode()).hexdigest()[:16]
    print("comment_id：", comment_id)
    # 发布人签名
    sign = None
    # 添加至数据库
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        cursor.execute("""INSERT INTO comments
            (object, description, date, source_cate, type, author_sign, id)
            VALUES (?, ?, ?, ?, ?, ?, ?)""",
                       (
                           object_id,
                           comment,
                           date,
                           'pireview',
                           'teacher',
                           sign,
                           comment_id,
                       )
                       )
        conn.commit()
    print('成功添加评价：', comment_id)
    return comment_id


def download(i):
    if pathlib.Path(f'./tmp/pi-review_pis_{i}.html').exists():
        return
    url = f"https://pi-review.com/pis/{i}"
    try:
        response = requests.get(url)

        if response.status_code != 200:
            print(f"{response.status_code}:{url} ", end='|')
            with open("./tmp/log.txt", "a") as f:
                f.write(f"{url} is {response.status_code}\n")
            return
        soup = BeautifulSoup(response.text, 'html.parser')
        content = soup.select_one('div', class_='wrapper')
        with open(f"./tmp/pi-review_pis_{i}.html", "w") as f:
            f.write(str(content))
    except Exception as e:
        print(f"{e}:{url} ", end='|')
        with open("./tmp/log.txt", "a") as f:
            f.write(f"{url} is {e}\n")


def download_muti(start, end):
    for i in range(start, end):
        download(i)


ALL_COUNT = 13098  # 2023-09-27 有 13089 个导师


def download_all():
    # 设置线程数
    num_threads = 4
    # 确定每个线程需要爬取的范围
    step = ALL_COUNT // num_threads
    ranges = [(i, i + step) for i in range(1, ALL_COUNT, step)]
    print(ranges)

    threads = []
    for start, end in ranges:
        thread = threading.Thread(target=download_muti, args=(start, end))
        thread.start()
        threads.append(thread)

    # 等待所有线程结束
    for thread in threads:
        thread.join()


def crawler_all():
    for i in range(1, ALL_COUNT):
        crawler(i)


if __name__ == "__main__":
    # 测试
    # download(8987)
    # crawler(8987)

    # 下载
    # download_all()

    # 数据处理 缺失的 i 也会重新下载
    crawler_all()
